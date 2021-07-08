CREATE TABLE users
(
    id       serial primary key,
    username varchar(512) not null,
    password varchar(512) not null
);

CREATE TABLE channels
(
    id      serial primary key,
    name    varchar(512) not null,
    url     varchar(512) not null,
    user_id integer      not null,
    FOREIGN KEY (user_id) REFERENCES users (id) on delete cascade on update cascade
);

CREATE TABLE items
(
    id         serial primary key,
    guid       text    null,
    title      text    null,
    url        text    null,
    content    text    null,
    read       boolean not null,
    channel_id integer not null,
    FOREIGN KEY (channel_id) REFERENCES channels (id) on delete cascade on update cascade
);


INSERT INTO users (id, username, password)
VALUES (1, 'root',
        '$argon2i$v=19$m=4096,t=2,p=1$bGVwZXRpdGNlcmVib3M$MCSscpJ5MlsPvEpK7J5203kQ2tmdXKF5s2Oo47aQOyg')