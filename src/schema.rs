#![allow(unused_imports)]

table! {
    use diesel::sql_types::*;
    use crate::model::User_role;

    categories (id) {
        id -> Int4,
        name -> Varchar,
        description -> Nullable<Text>,
        user_id -> Int4,
    }
}

table! {
    use diesel::sql_types::*;
    use crate::model::User_role;

    channel_users (channel_id, user_id) {
        channel_id -> Int4,
        user_id -> Int4,
    }
}

table! {
    use diesel::sql_types::*;
    use crate::model::User_role;

    channels (id) {
        id -> Int4,
        name -> Varchar,
        url -> Varchar,
    }
}

table! {
    use diesel::sql_types::*;
    use crate::model::User_role;

    items (id) {
        id -> Int4,
        guid -> Nullable<Text>,
        title -> Nullable<Text>,
        url -> Nullable<Text>,
        content -> Nullable<Text>,
        read -> Bool,
        channel_id -> Int4,
    }
}

table! {
    use diesel::sql_types::*;
    use crate::model::User_role;

    users (id) {
        id -> Int4,
        username -> Varchar,
        password -> Varchar,
        role -> User_role,
    }
}

joinable!(categories -> users (user_id));
joinable!(channel_users -> channels (channel_id));
joinable!(channel_users -> users (user_id));
joinable!(items -> channels (channel_id));

allow_tables_to_appear_in_same_query!(
    categories,
    channel_users,
    channels,
    items,
    users,
);
