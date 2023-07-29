ALTER TABLE channels
    ADD COLUMN disabled      BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN failure_count INT     NOT NULL DEFAULT 0;