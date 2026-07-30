CREATE TABLE assets (
    id INTEGER PRIMARY KEY NOT NULL,
    asset_base TEXT NOT NULL UNIQUE,
    -- Unique because lookup by description takes the first match, so two
    -- assets sharing one would make `transfer`/`burn` naming it a coin toss.
    -- The wallet rejects a duplicate before writing (asset_registry's
    -- `check_label` / `describes_another_asset`); this keeps the invariant
    -- from resting on every writer remembering to ask. SQLite permits many
    -- NULLs here, which is what an unlabelled discovered asset carries.
    description TEXT UNIQUE,
    desc_hash TEXT,
    own INTEGER NOT NULL DEFAULT 0,
    finalized INTEGER NOT NULL DEFAULT 0,
    issued_on_chain INTEGER NOT NULL DEFAULT 0
);
