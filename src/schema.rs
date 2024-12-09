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
    params (params_id) {
        params_id -> Integer,
        last_block_height -> Nullable<Integer>,
        last_block_hash -> Nullable<Binary>,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    notes,
    params,
);
