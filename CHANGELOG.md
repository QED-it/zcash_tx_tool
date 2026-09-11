# Changelog
All notable changes to this library will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this library adheres to Rust's notion of
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.0] - 2026-09-10
Post-NU6.2 release. Aligns the dependency set with Zebra v5.2.0 and is only
compatible with a Zebra node built from the commit pinned as `ZEBRA_COMMIT` in
`.github/workflows/zebra-test-ci.yaml`.

### Changed
- Aligned dependencies with Zebra v5.2.0: orchard 0.14, librustzcash 0.28
  (`zcash_primitives`, `zcash_proofs`, `zcash_transparent`, `zcash_protocol`,
  `zcash_encoding`), sapling-crypto 0.7, and the QED-it halo2 fork
  (`halo2_proofs` 0.3.2, `halo2_gadgets` 0.5.0, `halo2_poseidon`)
- Orchard proofs are now built for the post-NU6.2 Action circuit
  (`FixedPostNu6_2`), so transactions from this release are not verifiable by a
  pre-NU6.2 node
- `ZEBRA_COMMIT` in `.github/workflows/zebra-test-ci.yaml` is now documented as
  the source of truth for the compatible Zebra commit; references to the retired
  `zsa-integration-demo` branch are removed
- All CI, test, and Docker cargo invocations now pass `--locked`, so `Cargo.lock`
  drift fails the build instead of resolving silently

### Security
- Picks up the `halo2_gadgets` 0.5.0 fix for a critical vulnerability affecting
  its use in the Orchard circuit, and orchard 0.14's canonical proof-size
  enforcement (GHSA-2x4w-pxqw-58v9)

## [0.5.0] - 2026-05-18
### Added
- `test-issue-one` scenario, issuing a single asset (#45)
- `get-block-data` command and block header commitment support per ZIP-244
- `test-persistence-part1` / `test-persistence-part2` scenarios, covering wallet
  state, notes and commitment-tree positions surviving a process restart
- CI: pin and test against a specific Zebra commit (#80), cache the tx-tool
  build (#84), Rustfmt and Clippy lint workflows (#60), dependabot
  configuration (#55), and AWS auth for the run-tx-tool workflow (#76)

### Changed
- Upgraded Zebra to `2b036fd6` and migrated to librustzcash 0.26 / orchard 0.12 (#86)
- Updated librustzcash (#43)
- Clearer messages while creating transfer and burn transactions (#74)
- Removed the `.env` file and its configuration (#51)
- Assorted fixes (#63), plus dependency and GitHub Actions bumps

## [0.4.0] - 2025-11-06
### Added
- Asset finalization test (#42)

### Changed
- Updated librustzcash to revision `a996e53`, and again thereafter (#40, #41)
- Docker entry point now runs the compiled executable directly (#37)
- Release workflow pushes to ECR on a new release (#38), handling an
  empty user-supplied tag (#39)

## [0.3.2] - 2025-07-31
### Changed
- Version bump in `Cargo.toml`; no functional changes

## [0.3.1] - 2025-07-31
### Added
- Show git metadata when running `zcash_tx_tool`
- Embed `GIT_TAG` and `GIT_COMMIT` via build script
- Adjust acceptance tests for the new output

## [0.3.0] - 2025-06-03
### Added
- Support for the asset description hash in the issuance bundle
- Structures for describing transfer and burn information, ability to convert them into transactions
- Support for scenarios with arbitrary number of accounts
- Additional scenario for a three party test case
- Updated dependencies and the Zebra target branch to `zsa-integration-demo`

## [0.2.0] - 2025-02-28
### Added
- OrchardBundle enum
- first_issuance flag
- action-group-based serialization


## [0.1.0] - 2025-01-06
Initial release.
