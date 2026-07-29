//! Integration tests for the local ZSA asset registry.

use nonempty::NonEmpty;
use orchard::issuance::compute_asset_desc_hash;
use tempfile::TempDir;
use zcash_tx_tool::components::asset_registry;
use zcash_tx_tool::components::db;
use zcash_tx_tool::components::wallet::Wallet;

const SEED_PHRASE: &str = "fabric dilemma shift time border road fork license among uniform early laundry caution deer stamp";

fn desc_hash(desc: &str) -> [u8; 32] {
    compute_asset_desc_hash(&NonEmpty::from_slice(desc.as_bytes()).unwrap())
}

#[test]
fn registry_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("registry-test.sqlite");
    let mut conn = db::establish_connection(db_path.to_str().unwrap());
    let wallet = Wallet::new(&mut conn, SEED_PHRASE);

    // Own asset: full metadata.
    let hash_a = desc_hash("ASSET-A");
    let asset_a = wallet.asset_base_from_desc_hash(&hash_a);
    asset_registry::upsert_own_asset(&mut conn, &asset_a, "ASSET-A", &hash_a);

    let info = asset_registry::find_by_asset(&mut conn, &asset_a).expect("asset A registered");
    assert!(info.is_own());
    assert!(!info.is_finalized());
    assert_eq!(info.display_name(), "ASSET-A");
    assert_eq!(
        info.desc_hash.as_deref(),
        Some(hex::encode(hash_a).as_str())
    );
    assert_eq!(info.asset().unwrap(), asset_a);

    // Lookup by description and by AssetBase hex prefix.
    assert!(asset_registry::find_by_description(&mut conn, "ASSET-A").is_some());
    let base_hex = asset_registry::asset_base_hex(&asset_a);
    let matches = asset_registry::find_by_base_prefix(&mut conn, &base_hex[..8]);
    assert_eq!(matches.len(), 1);

    // Upsert is idempotent.
    asset_registry::upsert_own_asset(&mut conn, &asset_a, "ASSET-A", &hash_a);
    assert_eq!(asset_registry::list(&mut conn).len(), 1);

    // Discovered asset: base only, then user labels it.
    let hash_b = desc_hash("ASSET-B");
    let asset_b = wallet.asset_base_from_desc_hash(&hash_b);
    asset_registry::record_seen_asset(&mut conn, &asset_b);

    let info_b = asset_registry::find_by_asset(&mut conn, &asset_b).expect("asset B recorded");
    assert!(!info_b.is_own());
    assert!(info_b.description.is_none());
    assert!(info_b.display_name().starts_with("[unlabelled"));

    asset_registry::set_label(&mut conn, &asset_b, "Received Token");
    let info_b = asset_registry::find_by_asset(&mut conn, &asset_b).unwrap();
    assert_eq!(info_b.display_name(), "Received Token");

    // record_seen_asset must not clobber existing metadata.
    asset_registry::record_seen_asset(&mut conn, &asset_b);
    let info_b = asset_registry::find_by_asset(&mut conn, &asset_b).unwrap();
    assert_eq!(info_b.display_name(), "Received Token");

    // Finalization flag.
    asset_registry::set_finalized(&mut conn, &asset_a);
    let info = asset_registry::find_by_asset(&mut conn, &asset_a).unwrap();
    assert!(info.is_finalized());

    assert_eq!(asset_registry::list(&mut conn).len(), 2);
}

/// Labels annotate discovered assets; they must not overwrite an own asset's
/// issuance description or collide with a label already in use, since either
/// would make lookup by description wrong rather than merely confusing.
#[test]
fn labels_may_not_corrupt_the_registry() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("label-test.sqlite");
    let mut conn = db::establish_connection(db_path.to_str().unwrap());
    let wallet = Wallet::new(&mut conn, SEED_PHRASE);

    let hash = desc_hash("OWN-ASSET");
    let own = wallet.asset_base_from_desc_hash(&hash);
    asset_registry::upsert_own_asset(&mut conn, &own, "OWN-ASSET", &hash);

    let seen = wallet.asset_base_from_desc_hash(&desc_hash("SEEN-ASSET"));
    asset_registry::record_seen_asset(&mut conn, &seen);
    let other = wallet.asset_base_from_desc_hash(&desc_hash("OTHER-ASSET"));
    asset_registry::record_seen_asset(&mut conn, &other);

    // An own asset's description defines its AssetBase: not relabellable.
    assert!(asset_registry::check_label(&mut conn, &own, "Renamed").is_err());

    // A discovered asset takes a fresh label.
    assert!(asset_registry::check_label(&mut conn, &seen, "Received Token").is_ok());
    asset_registry::set_label(&mut conn, &seen, "Received Token");

    // That label is now taken — including by an own asset's description.
    assert!(asset_registry::check_label(&mut conn, &other, "Received Token").is_err());
    assert!(asset_registry::check_label(&mut conn, &other, "OWN-ASSET").is_err());
    // …but re-applying an asset's own label is a no-op, not a collision.
    assert!(asset_registry::check_label(&mut conn, &seen, "Received Token").is_ok());
    assert!(asset_registry::check_label(&mut conn, &other, "Another Token").is_ok());

    // The rejected labels left the registry untouched.
    assert_eq!(
        asset_registry::find_by_asset(&mut conn, &own)
            .unwrap()
            .display_name(),
        "OWN-ASSET"
    );
    assert!(
        asset_registry::find_by_asset(&mut conn, &other)
            .unwrap()
            .description
            .is_none()
    );
}

/// A wallet reset throws away chain state and rescans from scratch, so the
/// finalization it learned from issuance bundles must go with it — otherwise a
/// finalization that a reorg removed would linger. User labels must survive.
#[test]
fn reset_clears_chain_derived_finalization() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("reset-test.sqlite");
    let mut conn = db::establish_connection(db_path.to_str().unwrap());
    let mut wallet = Wallet::new(&mut conn, SEED_PHRASE);

    let hash = desc_hash("ASSET-FINAL");
    let own = wallet.asset_base_from_desc_hash(&hash);
    asset_registry::upsert_own_asset(&mut conn, &own, "ASSET-FINAL", &hash);
    asset_registry::set_finalized(&mut conn, &own);

    let seen = wallet.asset_base_from_desc_hash(&desc_hash("ASSET-SEEN"));
    asset_registry::record_seen_asset(&mut conn, &seen);
    asset_registry::set_label(&mut conn, &seen, "Received Token");
    asset_registry::set_finalized(&mut conn, &seen);

    wallet.reset(&mut conn);

    let info = asset_registry::find_by_asset(&mut conn, &own).expect("own asset kept");
    assert!(!info.is_finalized(), "finalization must be re-derived");
    assert!(info.is_own());
    assert_eq!(info.display_name(), "ASSET-FINAL");

    let info = asset_registry::find_by_asset(&mut conn, &seen).expect("discovered asset kept");
    assert!(!info.is_finalized(), "finalization must be re-derived");
    assert_eq!(info.display_name(), "Received Token", "label must survive");
}

/// Account indices reaching key derivation from user input must be validated,
/// not truncated to a different account or unwrapped into a panic.
#[test]
fn account_index_is_validated() {
    use orchard::keys::Scope::External;
    use zcash_tx_tool::components::wallet::MAX_ACCOUNT_INDEX;

    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("accounts-test.sqlite");
    let mut conn = db::establish_connection(db_path.to_str().unwrap());
    let mut wallet = Wallet::new(&mut conn, SEED_PHRASE);

    let addr = wallet
        .try_address_for_account(1, External)
        .expect("account 1 is derivable");
    assert_eq!(addr, wallet.address_for_account(1, External));

    // ZIP-32 account indices are hardened, so 2^31 and above are invalid.
    let mut rejected = vec![MAX_ACCOUNT_INDEX as usize + 1, usize::MAX];
    // 2^32 used to truncate to account 0 — a silent send to the wrong account.
    #[cfg(target_pointer_width = "64")]
    rejected.push(u32::MAX as usize + 1);

    for account in rejected {
        assert!(
            wallet.try_address_for_account(account, External).is_err(),
            "account index {} must be rejected",
            account
        );
    }
    assert!(
        wallet
            .try_address_for_account(MAX_ACCOUNT_INDEX as usize, External)
            .is_ok()
    );

    assert!(wallet.register_accounts(2).is_ok());
    assert!(
        wallet
            .register_accounts(MAX_ACCOUNT_INDEX as usize + 2)
            .is_err(),
        "an out-of-range account count must be rejected up front"
    );
}
