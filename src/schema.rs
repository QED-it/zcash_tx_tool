// @generated automatically by Diesel CLI.
#![allow(unused_qualifications)]
diesel::table! {
    notes (id) {
        id -> Integer,
        amount -> BigInt,
        tx_id -> Binary,
        tx_index -> Integer,
        merkle_path -> Binary,
        encrypted_note -> Binary,
    }
}
