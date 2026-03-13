CREATE TABLE IF NOT EXISTS block_data (
    height INTEGER PRIMARY KEY NOT NULL,
    hash TEXT NOT NULL,
    prev_hash TEXT NOT NULL
);
