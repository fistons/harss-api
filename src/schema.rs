#![allow(unused_imports)]

table! {
    use diesel::sql_types::*;
    use crate::model::user::User_role;

    channels (id) {
        id -> Int4,
        name -> Varchar,
        url -> Varchar,
    }
}

table! {
    use diesel::sql_types::*;
    use crate::model::user::User_role;

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
    use crate::model::user::User_role;

    users (id) {
        id -> Int4,
        username -> Varchar,
        password -> Varchar,
        role -> User_role,
    }
}

joinable!(items -> channels (channel_id));

allow_tables_to_appear_in_same_query!(
    channels,
    items,
    users,
);
