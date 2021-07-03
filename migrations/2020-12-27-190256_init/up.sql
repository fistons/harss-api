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
    FOREIGN KEY (user_id) REFERENCES users (id) on delete cascade on update cascade,
    UNIQUE(url, user_id)
);

CREATE TABLE items
(
    id         integer not null primary key autoincrement,
    guid       text    not null,
    title      text    null,
    url        text    null,
    content    text    null,
    read       boolean not null,
    channel_id integer not null,
    FOREIGN KEY (channel_id) REFERENCES channels (id) on delete cascade on update cascade
);


INSERT INTO users (id, username, password) VALUES (1, 'root', '$argon2i$v=19$m=4096,t=2,p=1$bGVwZXRpdGNlcmVib3M$MCSscpJ5MlsPvEpK7J5203kQ2tmdXKF5s2Oo47aQOyg')