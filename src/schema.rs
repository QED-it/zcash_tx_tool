// @generated automatically by Diesel CLI.

diesel::table! {
    issuance_data (id) {
        id -> Integer,
        asset -> Binary,
        amount -> BigInt,
        finalized -> Integer,
    }
}

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

diesel::allow_tables_to_appear_in_same_query!(
    issuance_data,
    notes,
);
