use orchard::{Address, Note};
use std::cmp::Ordering;
use orchard::keys::FullViewingKey;
use orchard::tree::MerklePath;

/// Internal newtype wrapper that allows us to use addresses as
/// BTreeMap keys.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub(crate) struct OrderedAddress(Address);

impl std::ops::Deref for OrderedAddress {
    type Target = Address;

    #[inline]
    fn deref(&self) -> &Address {
        &self.0
    }
}

impl OrderedAddress {
    pub(crate) fn new(addr: Address) -> Self {
        OrderedAddress(addr)
    }
}

impl PartialOrd for OrderedAddress {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrderedAddress {
    fn cmp(&self, other: &Self) -> Ordering {
        self.to_raw_address_bytes()
            .cmp(&other.to_raw_address_bytes())
    }
}


pub struct OrchardSpendInfo {
    fvk: FullViewingKey,
    note: Note,
    merkle_path: MerklePath,
}

impl OrchardSpendInfo {
    pub fn from_parts(fvk: FullViewingKey, note: Note, merkle_path: MerklePath) -> Self {
        OrchardSpendInfo {
            fvk,
            note,
            merkle_path,
        }
    }
}