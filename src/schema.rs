table! {
    channels (id) {
        id -> Integer,
        name -> Text,
        url -> Text,
    }
}

table! {
    items (id) {
        id -> Integer,
        title -> Text,
        url -> Text,
        content -> Text,
        channel_id -> Integer,
    }
}

joinable!(items -> channels (channel_id));

allow_tables_to_appear_in_same_query!(
    channels,
    items,
);
