CREATE TABLE notes (
    id INTEGER PRIMARY KEY NOT NULL,
    amount BigInt NOT NULL,
    tx_id BINARY(32) NOT NULL,
    action_index INTEGER NOT NULL,
    spend_tx_id BINARY(32) NOT NULL,
    spend_tx_index INTEGER NOT NULL,
    merkle_path BINARY NOT NULL,
    encrypted_note BINARY(580) NOT NULL,
    nullifier BINARY(32) NOT NULL
)