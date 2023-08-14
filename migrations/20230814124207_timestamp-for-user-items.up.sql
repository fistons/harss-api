ALTER TABLE users_items
    ADD COLUMN
        added_timestamp timestamptz not null default now();