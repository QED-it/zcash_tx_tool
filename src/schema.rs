// @generated automatically by Diesel CLI.

diesel::table! {
    notes (id) {
        id -> Integer,
        amount -> BigInt,
        asset -> Binary,
        tx_id -> Binary,
        action_index -> Integer,
        position -> BigInt,
        memo -> Binary,
        rho -> Binary,
        nullifier -> Binary,
        rseed -> Binary,
        recipient_address -> Binary,
        spend_tx_id -> Nullable<Binary>,
        spend_action_index -> Integer,
    }
}

diesel::table! {
    /// Stored block data used for syncing and reorg detection.
    ///
    /// Note: `tx_hex_json` stores a JSON array of hex-encoded transaction bytes.
    block_data (height) {
        height -> Integer,
        hash -> Text,
        prev_hash -> Text,
        tx_hex_json -> Text,
    }
}
