CREATE TABLE IF NOT EXISTS channels_errors
(
    id              SERIAL PRIMARY KEY,
    channel_id      integer     not null,
    error_timestamp timestamptz not null default now(),
    error_reason    TEXT,
    FOREIGN KEY (channel_id) REFERENCES channels (id) ON DELETE CASCADE ON UPDATE CASCADE
);
