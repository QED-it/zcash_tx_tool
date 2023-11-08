use orchard::Address;
use zcash_client_backend::address::RecipientAddress;
use zcash_primitives::consensus::TEST_NETWORK;

pub(crate) fn orchard_address_from_ua(ua_address: &String) -> Address {

    // TODO implement better address parsing?
    // TODO take NETWORK from config

    match RecipientAddress::decode(&TEST_NETWORK, ua_address) {
        Some(RecipientAddress::Unified(ua)) => {
            ua.orchard().unwrap().clone()
        }
        Some(_) => {
            panic!(
                "{} did not decode to a unified address value.",
                &ua_address.as_str()
            );
        }
        None => {
            panic!(
                "Failed to decode unified address from test vector: {}",
                &ua_address.as_str()
            );
        }
    }
}