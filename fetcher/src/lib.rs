use sea_orm::entity::*;

use entity::channels::Entity as Channel;

#[tracing::instrument(skip_all)]
pub async fn fetch(client: reqwest::Client, pool: sea_orm::DatabaseConnection) {
    let channels = Channel::find().all(&pool).await.unwrap();

    let mut tasks = vec![];
    for channel in channels {
        let client = client.clone();
        let task = tokio::task::spawn(async move {
            client
                .get(channel.url)
                .send()
                .await
                .unwrap()
                .error_for_status()
                .unwrap()
        });

        tasks.push(task);
    }

    for task in tasks {
        //TODO: Do something with the response
        let _response = task.await.unwrap();
    }
}
