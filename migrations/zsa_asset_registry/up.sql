CREATE TABLE assets (
    id INTEGER PRIMARY KEY NOT NULL,
    asset_base TEXT NOT NULL UNIQUE,
    description TEXT,
    desc_hash TEXT,
    own INTEGER NOT NULL DEFAULT 0,
    finalized INTEGER NOT NULL DEFAULT 0
);
