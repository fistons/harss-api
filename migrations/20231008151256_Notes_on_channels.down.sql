ALTER TABLE channel_users
    DROP COLUMN IF EXISTS name,
    DROP COLUMN IF EXISTS notes;