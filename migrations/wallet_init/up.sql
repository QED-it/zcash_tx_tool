CREATE TABLE notes (
    id INTEGER PRIMARY KEY NOT NULL,
    amount BigInt NOT NULL,
    asset BINARY(32) NOT NULL,
    tx_id BINARY(32) NOT NULL,
    action_index INTEGER NOT NULL,
    position BigInt NOT NULL,
    memo BINARY(512) NOT NULL,
    rho BINARY(32) NOT NULL,
    nullifier BINARY(32) NOT NULL,
    rseed BINARY(32) NOT NULL,
    recipient_address BINARY(43) NOT NULL,
    spend_tx_id BINARY(32),
    spend_action_index INTEGER NOT NULL,
    origin_block_height INTEGER NOT NULL DEFAULT 0,
    spend_block_height INTEGER
);

CREATE TABLE IF NOT EXISTS block_data (
    height INTEGER PRIMARY KEY NOT NULL,
    hash TEXT NOT NULL,
    prev_hash TEXT NOT NULL,
    tx_data_json TEXT NOT NULL DEFAULT '[]'
);
