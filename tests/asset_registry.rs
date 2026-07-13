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
