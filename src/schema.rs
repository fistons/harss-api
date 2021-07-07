table! {
    channels (id) {
        id -> Int4,
        name -> Varchar,
        url -> Varchar,
        user_id -> Int4,
    }
}

table! {
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
    users (id) {
        id -> Int4,
        username -> Varchar,
        password -> Varchar,
    }
}

joinable!(channels -> users (user_id));
joinable!(items -> channels (channel_id));

allow_tables_to_appear_in_same_query!(
    channels,
    items,
    users,
);
