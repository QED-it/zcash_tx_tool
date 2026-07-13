//! Local registry of ZSA assets known to this wallet.
//!
//! Assets issued from this wallet are stored with their full metadata
//! (description string and description hash). Assets discovered in received
//! notes are recorded by their `AssetBase` only — on-chain there is nothing
//! else to learn about a shielded asset — and can be labelled by the user
//! with a human-readable name.

use diesel::prelude::*;
use orchard::note::AssetBase;

use crate::schema::assets::dsl as a;

/// A row of the `assets` registry table.
#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = crate::schema::assets)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct AssetInfo {
    pub id: i32,
    /// Hex-encoded 32-byte AssetBase (the on-chain asset identifier).
    pub asset_base: String,
    /// Human-readable asset description; `None` for discovered assets.
    pub description: Option<String>,
    /// Hex-encoded 32-byte asset description hash (known for own assets).
    pub desc_hash: Option<String>,
    /// 1 if the asset was issued with this wallet's issuance key.
    pub own: i32,
    /// 1 if the asset supply has been finalized (no further issuance).
    pub finalized: i32,
}

impl AssetInfo {
    pub fn asset(&self) -> Option<AssetBase> {
        let bytes: [u8; 32] = hex::decode(&self.asset_base).ok()?.try_into().ok()?;
        AssetBase::from_bytes(&bytes).into()
    }

    /// Display name: description when known, otherwise a shortened AssetBase.
    pub fn display_name(&self) -> String {
        match &self.description {
            Some(desc) => desc.clone(),
            None => format!("[unlabelled {}…]", &self.asset_base[..8]),
        }
    }

    pub fn is_own(&self) -> bool {
        self.own != 0
    }

    pub fn is_finalized(&self) -> bool {
        self.finalized != 0
    }
}

pub fn asset_base_hex(asset: &AssetBase) -> String {
    hex::encode(asset.to_bytes())
}

/// All registry entries, own assets first, then by id.
pub fn list(conn: &mut SqliteConnection) -> Vec<AssetInfo> {
    a::assets
        .order((a::own.desc(), a::id.asc()))
        .select(AssetInfo::as_select())
        .load(conn)
        .expect("Error loading asset registry")
}

pub fn find_by_asset(conn: &mut SqliteConnection, asset: &AssetBase) -> Option<AssetInfo> {
    a::assets
        .filter(a::asset_base.eq(asset_base_hex(asset)))
        .select(AssetInfo::as_select())
        .first(conn)
        .optional()
        .expect("Error querying asset registry")
}

pub fn find_by_description(conn: &mut SqliteConnection, desc: &str) -> Option<AssetInfo> {
    a::assets
        .filter(a::description.eq(desc))
        .select(AssetInfo::as_select())
        .first(conn)
        .optional()
        .expect("Error querying asset registry")
}

/// Find registry entries whose AssetBase hex starts with the given prefix.
pub fn find_by_base_prefix(conn: &mut SqliteConnection, prefix: &str) -> Vec<AssetInfo> {
    a::assets
        .filter(a::asset_base.like(format!("{}%", prefix)))
        .select(AssetInfo::as_select())
        .load(conn)
        .expect("Error querying asset registry")
}

/// Record an asset issued by this wallet (idempotent).
pub fn upsert_own_asset(
    conn: &mut SqliteConnection,
    asset: &AssetBase,
    description: &str,
    desc_hash: &[u8; 32],
) {
    let base = asset_base_hex(asset);
    diesel::insert_into(a::assets)
        .values((
            a::asset_base.eq(&base),
            a::description.eq(description),
            a::desc_hash.eq(hex::encode(desc_hash)),
            a::own.eq(1),
        ))
        .on_conflict(a::asset_base)
        .do_update()
        .set((
            a::description.eq(description),
            a::desc_hash.eq(hex::encode(desc_hash)),
            a::own.eq(1),
        ))
        .execute(conn)
        .expect("Error upserting asset");
}

/// Record an asset seen in a received note; keeps existing metadata if present.
pub fn record_seen_asset(conn: &mut SqliteConnection, asset: &AssetBase) {
    diesel::insert_into(a::assets)
        .values(a::asset_base.eq(asset_base_hex(asset)))
        .on_conflict(a::asset_base)
        .do_nothing()
        .execute(conn)
        .expect("Error recording asset");
}

/// Attach a user-supplied label to a discovered asset.
pub fn set_label(conn: &mut SqliteConnection, asset: &AssetBase, label: &str) {
    diesel::update(a::assets.filter(a::asset_base.eq(asset_base_hex(asset))))
        .set(a::description.eq(label))
        .execute(conn)
        .expect("Error labelling asset");
}

/// Mark an asset's supply as finalized.
pub fn set_finalized(conn: &mut SqliteConnection, asset: &AssetBase) {
    diesel::update(a::assets.filter(a::asset_base.eq(asset_base_hex(asset))))
        .set(a::finalized.eq(1))
        .execute(conn)
        .expect("Error finalizing asset");
}

/// Scan the notes table for assets not yet present in the registry and record
/// them as discovered. Returns the newly recorded assets. Run after sync.
pub fn discover_assets_from_notes(conn: &mut SqliteConnection) -> Vec<AssetBase> {
    use crate::schema::notes::dsl as n;
    let asset_blobs: Vec<Vec<u8>> = n::notes
        .select(n::asset)
        .distinct()
        .load(conn)
        .expect("Error scanning note assets");

    let mut discovered = Vec::new();
    for blob in asset_blobs {
        let bytes: [u8; 32] = match blob.as_slice().try_into() {
            Ok(b) => b,
            Err(_) => continue,
        };
        let asset: Option<AssetBase> = AssetBase::from_bytes(&bytes).into();
        let Some(asset) = asset else { continue };
        if bool::from(asset.is_zatoshi()) {
            continue; // native ZEC needs no registry entry
        }
        if find_by_asset(conn, &asset).is_none() {
            record_seen_asset(conn, &asset);
            discovered.push(asset);
        }
    }
    discovered
}
