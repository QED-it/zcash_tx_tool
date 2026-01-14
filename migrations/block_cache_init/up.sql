CREATE TABLE IF NOT EXISTS block_cache (
    height INTEGER PRIMARY KEY NOT NULL,
    hash TEXT NOT NULL,
    prev_hash TEXT NOT NULL,
    tx_hex_json TEXT NOT NULL DEFAULT '[]'
);


