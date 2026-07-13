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
    /// Stored block data used for resumable sync.
    block_data (height) {
        height -> Integer,
        hash -> Text,
    }
}

diesel::table! {
    /// Persisted wallet state: commitment tree, last synced block height/hash.
    wallet_state (id) {
        id -> Integer,
        commitment_tree_json -> Text,
        last_block_height -> Integer,
        last_block_hash -> Text,
    }
}

diesel::table! {
    /// Registry of ZSA assets known to this wallet, keyed by hex-encoded
    /// AssetBase. `description`/`desc_hash` are only known for assets issued
    /// (or labelled) locally; assets discovered in received notes have NULL
    /// description until the user labels them.
    assets (id) {
        id -> Integer,
        asset_base -> Text,
        description -> Nullable<Text>,
        desc_hash -> Nullable<Text>,
        own -> Integer,
        finalized -> Integer,
    }
}
