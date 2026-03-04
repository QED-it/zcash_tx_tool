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
        origin_block_height -> Integer,
        spend_block_height -> Nullable<Integer>,
    }
}

diesel::table! {
    /// Stored block headers for syncing and reorg detection.
    block_data (height) {
        height -> Integer,
        hash -> Text,
        prev_hash -> Text,
    }
}
