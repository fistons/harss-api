CREATE TABLE items
(
    id      integer not null primary key autoincrement,
    title   text    not null,
    url     text    not null,
    content text    not null,
    channel_id integer not null,
    FOREIGN KEY (channel_id) REFERENCES channels(id) on delete cascade on update cascade 
)