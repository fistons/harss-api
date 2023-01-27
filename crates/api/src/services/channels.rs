use chrono::{DateTime, Utc};
use sea_orm::sea_query::{Alias, Expr, SimpleExpr};
use sea_orm::DatabaseConnection;
use sea_orm::{entity::*, query::*, DbErr};

use entity::channels::Entity as Channel;
use entity::users_items::Entity as UsersItems;
use entity::{channel_users, channels, users_items};
use RssParsingError::NonOkStatus;

use crate::model::{HttpChannel, HttpNewChannel, HttpUserChannel, PagedResult};
use crate::services::{RssParsingError, ServiceError};

#[derive(Clone)]
pub struct ChannelService {
    db: DatabaseConnection,
    client: reqwest::Client,
}

/// Generate a select statement for channel and user
fn user_channel_select_statement() -> Select<Channel> {
    Channel::find()
        .join(JoinType::RightJoin, channels::Relation::ChannelUsers.def())
        .join(JoinType::LeftJoin, channels::Relation::UsersItems.def())
        .column_as(users_items::Column::ItemId.count(), "items_count")
        .column_as(
            Expr::expr(
                Into::<SimpleExpr>::into(Expr::col(users_items::Column::Read))
                    .cast_as(Alias::new("integer")),
            )
            .sum(),
            "items_read",
        )
}

impl ChannelService {
    pub fn new(db: DatabaseConnection) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("rss-aggregator checker (+https://github.com/fistons/rss-aggregator)")
            .build()
            .expect("Could not build client");

        ChannelService { db, client }
    }

    #[tracing::instrument(skip(self))]
    pub async fn select_by_id_and_user_id(
        &self,
        chan_id: i32,
        user_id: i32,
    ) -> Result<Option<HttpUserChannel>, ServiceError> {
        Ok(user_channel_select_statement()
            .filter(channel_users::Column::UserId.eq(user_id))
            .filter(channel_users::Column::ChannelId.eq(chan_id))
            .group_by(channels::Column::Id)
            .into_model::<HttpUserChannel>()
            .one(&self.db)
            .await?)
    }

    #[tracing::instrument(skip(self))]
    pub async fn mark_channel_as_read(&self, chan_id: i32, user_id: i32) -> Result<(), DbErr> {
        UsersItems::update_many()
            .col_expr(users_items::Column::Read, Expr::value(true))
            .filter(users_items::Column::ChannelId.eq(chan_id))
            .filter(users_items::Column::UserId.eq(user_id))
            .exec(&self.db)
            .await?;

        tracing::debug!("Chanel {} marked as read for user {}", chan_id, user_id);

        Ok(())
    }

    ///  Select all the channels of a user, along side the total number of items
    #[tracing::instrument(skip(self))]
    pub async fn select_page_by_user_id(
        &self,
        u_id: i32,
        page: u64,
        page_size: u64,
    ) -> Result<PagedResult<HttpUserChannel>, ServiceError> {
        let channel_paginator = user_channel_select_statement()
            .filter(channel_users::Column::UserId.eq(u_id))
            .group_by(channels::Column::Id)
            .group_by(channel_users::Column::RegistrationTimestamp)
            .order_by_desc(channel_users::Column::RegistrationTimestamp)
            .into_model::<HttpUserChannel>()
            .paginate(&self.db, page_size);

        let total_items_and_pages = channel_paginator.num_items_and_pages().await?;
        let total_pages = total_items_and_pages.number_of_pages;
        let content = channel_paginator.fetch_page(page - 1).await?;
        let elements_number = content.len();

        Ok(PagedResult {
            content,
            page,
            page_size,
            total_pages,
            elements_number,
            total_items: total_items_and_pages.number_of_items,
        })
    }

    /// # Select all the channels
    #[tracing::instrument(skip(self))]
    pub async fn select_all_enabled(&self) -> Result<Vec<HttpChannel>, ServiceError> {
        Ok(Channel::find()
            .filter(channels::Column::Disabled.eq(false))
            .into_model::<HttpChannel>()
            .all(&self.db)
            .await?)
    }

    #[tracing::instrument(skip(self))]
    pub async fn select_all_enabled_by_user_id(
        &self,
        user_id: i32,
    ) -> Result<Vec<HttpChannel>, ServiceError> {
        Ok(Channel::find()
            .join(JoinType::RightJoin, channels::Relation::ChannelUsers.def())
            .filter(channel_users::Column::UserId.eq(user_id))
            .filter(channels::Column::Disabled.eq(false))
            .into_model::<HttpChannel>()
            .all(&self.db)
            .await?)
    }

    /// # Create a new channel in the database
    #[tracing::instrument(skip(self))]
    async fn create_new_channel(
        &self,
        new_channel: &HttpNewChannel,
    ) -> Result<channels::Model, ServiceError> {
        // Check that the feed is a parsable RSS feed
        check_feed(&self.client, &new_channel.url).await?;

        let channel = channels::ActiveModel {
            id: NotSet,
            name: Set(new_channel.name.to_owned()),
            url: Set(new_channel.url.to_owned()),
            last_update: NotSet,
            registration_timestamp: Set(Utc::now().into()),
            disabled: Set(false),
            failure_count: Set(0),
        };

        Ok(channel.insert(&self.db).await?)
    }

    /// Create or linked an existing channel to a user
    #[tracing::instrument(skip(self))]
    pub async fn create_or_link_channel(
        &self,
        new_channel: HttpNewChannel,
        other_user_id: i32,
    ) -> Result<channels::Model, ServiceError> {
        let channel = match Channel::find()
            .filter(channels::Column::Url.eq(&*new_channel.url))
            .one(&self.db)
            .await?
        {
            Some(found) => found,
            None => self.create_new_channel(&new_channel).await?,
        };

        let channel_user = channel_users::ActiveModel {
            channel_id: Set(channel.id),
            user_id: Set(other_user_id),
            registration_timestamp: Set(Utc::now().into()),
        };

        match channel_user.insert(&self.db).await {
            Ok(_) => Ok(channel),
            Err(DbErr::Query(x)) => {
                tracing::error!(
                    "Channel {} for user {} already inserted: {x}",
                    channel.name,
                    other_user_id
                );
                Ok(channel)
            }
            Err(x) => Err(x.into()),
        }
    }

    /// Update the last fetched timestamp of a channel
    #[tracing::instrument(skip(self))]
    pub async fn update_last_fetched(
        &self,
        channel_id: i32,
        date: DateTime<Utc>,
    ) -> Result<(), ServiceError> {
        Channel::update_many()
            .col_expr(channels::Column::LastUpdate, Expr::value(date))
            .filter(channels::Column::Id.eq(channel_id))
            .exec(&self.db)
            .await?;

        Ok(())
    }

    /// Enable a channel and reset it's failure count
    #[tracing::instrument(skip(self))]
    pub async fn enable_channel(&self, id: i32) -> Result<(), DbErr> {
        Channel::update_many()
            .col_expr(channels::Column::Disabled, Expr::value(false))
            .col_expr(channels::Column::FailureCount, Expr::value(0))
            .filter(channels::Column::Id.eq(id))
            .exec(&self.db)
            .await?;

        tracing::debug!("Chanel {} enabled", id);

        Ok(())
    }
}

/// Check that the feed is correct
#[tracing::instrument(skip(client))]
async fn check_feed(client: &reqwest::Client, url: &str) -> Result<(), RssParsingError> {
    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        return Err(NonOkStatus(response.status().as_u16()));
    }
    let feed_content = response.bytes().await?;
    feed_rs::parser::parse(&feed_content[..])?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    #[tokio::test]
    async fn test_check_feed_is_ok() {
        let mock = MockServer::start().await;
        let client = reqwest::Client::default();

        let valid_response = r#"
        <?xml version="1.0" encoding="UTF-8" ?>
        <rss version="2.0">
        <channel>
          <title>W3Schools Home Page</title>
          <link>https://www.w3schools.com</link>
          <description>Free web building tutorials</description>
          <item>
            <title>RSS Tutorial</title>
            <link>https://www.w3schools.com/xml/xml_rss.asp</link>
            <description>New RSS tutorial on W3Schools</description>
          </item>
        </channel>
        "#;

        let response = ResponseTemplate::new(200).set_body_raw(valid_response, "application/xml");

        Mock::given(method("GET"))
            .respond_with(response)
            .expect(1)
            .mount(&mock)
            .await;

        assert!(check_feed(&client, &mock.uri()).await.is_ok());
    }

    #[tokio::test]
    async fn test_check_feed_non_200() {
        let mock = MockServer::start().await;
        let client = reqwest::Client::default();

        let response = ResponseTemplate::new(404);

        Mock::given(method("GET"))
            .respond_with(response)
            .expect(1)
            .mount(&mock)
            .await;

        assert!(matches!(
            check_feed(&client, &mock.uri()).await,
            Err(RssParsingError::NonOkStatus { .. })
        ));
    }

    #[tokio::test]
    async fn test_check_feed_invalid_rss() {
        let mock = MockServer::start().await;
        let client = reqwest::Client::default();

        let response = ResponseTemplate::new(200).set_body_raw("rss lol", "application/xml");
        Mock::given(method("GET"))
            .respond_with(response)
            .expect(1)
            .mount(&mock)
            .await;

        assert!(matches!(
            check_feed(&client, &mock.uri()).await,
            Err(RssParsingError::ParseFeedError { .. })
        ));
    }
}
