CREATE TABLE items
(
    id         integer not null primary key autoincrement,
    guid       text    null,
    title      text    null,
    url        text    null,
    content    text    null,
    read       boolean not null,
    channel_id integer not null,
    FOREIGN KEY (channel_id) REFERENCES channels (id) on delete cascade on update cascade
)