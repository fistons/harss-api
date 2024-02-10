ALTER TABLE IF EXISTS users ADD COLUMN IF NOT EXISTS email text null;
CREATE INDEX IF NOT EXISTS users_email ON users (email);
