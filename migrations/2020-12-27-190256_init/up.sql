CREATE TABLE users (
    id integer not null primary key autoincrement,
    username varchar(512) not null,
    password varchar(512) not null
);

CREATE TABLE channels (
    id integer not null primary key autoincrement,
    name varchar(512) not null,
    url varchar(512) not null,
    user_id integer not null,
    FOREIGN KEY (user_id) REFERENCES users (id) on delete cascade on update cascade
);

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
);


INSERT INTO users (id, username, password) VALUES (1, 'root', 'root')