table! {
    channels (id) {
        id -> Integer,
        name -> Text,
        url -> Text,
        user_id -> Integer,
    }
}

table! {
    items (id) {
        id -> Integer,
        guid -> Nullable<Text>,
        title -> Nullable<Text>,
        url -> Nullable<Text>,
        content -> Nullable<Text>,
        read -> Bool,
        channel_id -> Integer,
    }
}

table! {
    users (id) {
        id -> Integer,
        username -> Text,
        password -> Text,
    }
}

joinable!(channels -> users (user_id));
joinable!(items -> channels (channel_id));

allow_tables_to_appear_in_same_query!(
    channels,
    items,
    users,
);
