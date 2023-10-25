// @generated automatically by Diesel CLI.

diesel::table! {
    notes (id) {
        id -> Integer,
        amount -> BigInt,
        tx_id -> Binary,
        action_index -> Integer,
        spend_tx_id -> Binary,
        spend_tx_index -> Integer,
        merkle_path -> Binary,
        encrypted_note -> Binary,
        nullifier -> Binary,
    }
}
