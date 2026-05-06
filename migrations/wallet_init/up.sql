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
    spend_action_index INTEGER NOT NULL
);

CREATE TABLE block_data (
    height INTEGER PRIMARY KEY NOT NULL,
    hash TEXT NOT NULL
);

CREATE TABLE wallet_state (
    id INTEGER PRIMARY KEY NOT NULL DEFAULT 1,
    commitment_tree_json TEXT NOT NULL,
    last_block_height INTEGER NOT NULL,
    last_block_hash TEXT NOT NULL
);
