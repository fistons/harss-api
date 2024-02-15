ALTER TABLE IF EXISTS users DROP CONSTRAINT users_email_unique;
DROP INDEX IF EXISTS users_email_unique; 
