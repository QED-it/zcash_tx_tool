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
        origin_block_height -> Integer,
        spend_block_height -> Nullable<Integer>,
    }
}

diesel::table! {
    /// Stored block data used for syncing and reorg detection.
    ///
    /// Note: `tx_data_json` stores a JSON array of hex-encoded transaction bytes.
    block_data (height) {
        height -> Integer,
        hash -> Text,
        prev_hash -> Text,
        tx_data_json -> Text,
    }
}
