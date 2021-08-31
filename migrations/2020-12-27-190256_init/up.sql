CREATE TYPE user_role AS ENUM ('basic', 'admin');
CREATE TABLE users
(
    id       serial primary key,
    username varchar(512) not null unique,
    password varchar(512) not null,
    role     user_role     not null
);

-- Ok let's use high start sequence, so our root user won't interfere
ALTER SEQUENCE users_id_seq RESTART WITH 666;


CREATE TABLE categories
(
    id serial primary key,
    name varchar(64) not null,
    description text,
    user_id integer not null,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE ON UPDATE CASCADE 
);

CREATE TABLE channels
(
    id      serial primary key,
    name    varchar(512) not null,
    url     varchar(512) not null
    --user_id integer      not null,
    --FOREIGN KEY (user_id) REFERENCES users (id) on delete cascade on update cascade
);

CREATE UNIQUE INDEX ON channels (url);

CREATE TABLE channel_users (
    channel_id integer not null,
    user_id integer not null,
    PRIMARY KEY (channel_id, user_id),
    FOREIGN KEY (channel_id) REFERENCES channels (id) ON DELETE CASCADE ON UPDATE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE ON UPDATE CASCADE
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


INSERT INTO users (id, username, password, role)
VALUES (1, 'root',
        '$argon2i$v=19$m=4096,t=2,p=1$bGVwZXRpdGNlcmVib3M$MCSscpJ5MlsPvEpK7J5203kQ2tmdXKF5s2Oo47aQOyg',
        'admin')