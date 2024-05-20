CREATE TABLE notes (
    id INTEGER PRIMARY KEY NOT NULL,
    amount BigInt NOT NULL,
    asset BINARY(32) NOT NULL,
    tx_id BINARY(32) NOT NULL,
    action_index INTEGER NOT NULL,
    position BigInt NOT NULL,
    memo BINARY(512) NOT NULL,
    nullifier BINARY(32) NOT NULL,
    rseed BINARY(32) NOT NULL,
    recipient_address BINARY(43) NOT NULL,
    spend_tx_id BINARY(32),
    spend_action_index INTEGER NOT NULL
)