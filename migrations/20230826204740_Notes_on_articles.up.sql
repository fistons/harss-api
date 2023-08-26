ALTER TABLE users_items
    ADD COLUMN IF NOT EXISTS
        notes varchar(5000) null;