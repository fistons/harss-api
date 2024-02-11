CREATE UNIQUE INDEX IF NOT EXISTS users_email_unique ON users (email);
ALTER TABLE IF EXISTS users ADD CONSTRAINT users_email_unique UNIQUE USING INDEX users_email_unique;
