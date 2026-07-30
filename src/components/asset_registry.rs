//! Local registry of ZSA assets known to this wallet.
//!
//! Assets issued from this wallet are stored with their full metadata
//! (description string and description hash). Assets discovered in received
//! notes are recorded by their `AssetBase` only — on-chain there is nothing
//! else to learn about a shielded asset — and can be labelled by the user
//! with a human-readable name.

use diesel::prelude::*;
use orchard::note::AssetBase;
use std::collections::HashSet;

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
    ///
    /// Falls back to the stored value whole if it is shorter than the prefix
    /// we would show: a corrupt row in a local database should not take down
    /// every command that lists assets.
    pub fn display_name(&self) -> String {
        match &self.description {
            Some(desc) => desc.clone(),
            None => format!(
                "[unlabelled {}…]",
                self.asset_base.get(..8).unwrap_or(&self.asset_base)
            ),
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

/// Whether `label` may be attached to `asset`, or why not.
///
/// Two labels would corrupt the registry rather than annotate it:
///
/// * An own asset's `description` is the issuance description whose hash
///   defines its `AssetBase` (ZIP 227) — protocol data, not a nickname.
///   Overwriting it would leave the registry describing an asset that the
///   description no longer identifies.
/// * A description shared by two assets makes lookup by description
///   ambiguous, and `find_by_description` would silently resolve to whichever
///   row came first — the same hazard [`find_by_base_prefix`] callers reject
///   for ambiguous hex prefixes.
pub fn check_label(
    conn: &mut SqliteConnection,
    asset: &AssetBase,
    label: &str,
) -> Result<(), String> {
    let base = asset_base_hex(asset);
    if let Some(info) = find_by_asset(conn, asset) {
        if info.is_own() {
            return Err(format!(
                "asset {} was issued by this wallet: its description '{}' defines its \
                 AssetBase and cannot be relabelled",
                base,
                info.display_name()
            ));
        }
    }
    match find_by_description(conn, label) {
        // Relabelling an asset with the name it already has is a no-op.
        Some(other) if other.asset_base != base => Err(format!(
            "'{}' already labels asset {} — pick another name, so that referring to \
             an asset by description stays unambiguous",
            label, other.asset_base
        )),
        _ => Ok(()),
    }
}

/// Attach a user-supplied label to a discovered asset. Callers should consult
/// [`check_label`] first: this is a plain setter.
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

/// Clear the finalized flag on every registry entry.
///
/// Finalization is chain-derived: it is learned from on-chain issuance bundles
/// during sync. Whenever the wallet throws away its chain state and rescans
/// from scratch (`clean`, or reorg/divergence recovery), the flags must be
/// dropped so they are re-derived from the chain that actually exists —
/// otherwise a finalization that was reorged out would linger, misreporting
/// supply state and locally blocking further issuance. User-authored fields
/// (descriptions/labels) and the own-asset marker are kept.
pub fn clear_finalized_flags(conn: &mut SqliteConnection) {
    diesel::update(a::assets)
        .set(a::finalized.eq(0))
        .execute(conn)
        .expect("Error clearing asset finalization flags");
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

    // Read the registry's contents once and test membership in memory: this
    // runs on every sync, and a query per distinct asset adds up.
    let mut known: HashSet<String> = a::assets
        .select(a::asset_base)
        .load(conn)
        .expect("Error loading known assets")
        .into_iter()
        .collect();

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
        // `insert` reports whether the asset was absent, i.e. newly discovered.
        if known.insert(asset_base_hex(&asset)) {
            record_seen_asset(conn, &asset);
            discovered.push(asset);
        }
    }
    discovered
}

#[cfg(test)]
mod tests {
    use super::AssetInfo;

    fn row(asset_base: &str, description: Option<&str>) -> AssetInfo {
        AssetInfo {
            id: 1,
            asset_base: asset_base.to_string(),
            description: description.map(str::to_string),
            desc_hash: None,
            own: 0,
            finalized: 0,
        }
    }

    #[test]
    fn display_name_survives_a_short_asset_base() {
        // Registry rows are written as 64-char hex…
        let full = "a".repeat(64);
        assert_eq!(
            row(&full, None).display_name(),
            format!("[unlabelled {}…]", &full[..8])
        );
        // …but a corrupt local row must not panic the commands listing it.
        assert_eq!(row("abc", None).display_name(), "[unlabelled abc…]");
        assert_eq!(row("", None).display_name(), "[unlabelled …]");
        // A described asset never consults the AssetBase at all.
        assert_eq!(row("abc", Some("My Token")).display_name(), "My Token");
    }
}
