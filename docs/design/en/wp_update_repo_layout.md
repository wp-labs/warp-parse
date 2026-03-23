# `wp-update` Repository Layout

- Status: Draft
- Scope: `https://github.com/wp-labs/wp-update.git`

## Background

`wp-update-core`, `wp-self-update`, and `wp-installer` have already been split out of the main `warp-parse` functionality. They now need independent publishing and must serve multiple binaries.

They need an independent lifecycle, but not three separate repositories.

## Decision

Use a dedicated `wp-update` repository and keep a single Rust workspace inside it.

The repository contains three crates:

- `wp-update-core`
- `wp-self-update`
- `wp-installer`

Do not split them into three repositories.

## Why

- The dependency chain is explicit: `wp-installer -> wp-self-update -> wp-update-core`
- API changes usually require coordinated edits and joint verification
- Publishing already has an ordered flow, so one repository is easier to manage
- These capabilities should no longer be tied to the `warp-parse` repository lifecycle
- Reuse by other products should not require depending on the whole `warp-parse` repository

## Suggested Layout

```text
wp-update/
├── .github/
├── CHANGELOG.md
├── Cargo.toml
├── README.md
└── crates/
    ├── wp-update-core/
    │   ├── Cargo.toml
    │   ├── README.md
    │   └── src/
    ├── wp-self-update/
    │   ├── Cargo.toml
    │   ├── README.md
    │   └── src/
    └── wp-installer/
        ├── Cargo.toml
        ├── README.md
        └── src/
```

## Suggested Root `Cargo.toml`

```toml
[workspace]
members = [
    "crates/wp-update-core",
    "crates/wp-self-update",
    "crates/wp-installer",
]
resolver = "2"
```

The root manifest should only own workspace configuration, not a publishable package.

## Release Strategy

The release order is fixed:

1. `wp-update-core`
2. `wp-self-update`
3. `wp-installer`

Each crate keeps its own version. Version numbers do not need to stay identical.

## Repository Ownership

- `wp-update-core`: channel model, manifest parsing, version comparison, shared types
- `wp-self-update`: download, verify, replace, rollback, and update execution
- `wp-installer`: first-install and installation entrypoint orchestration

Product repositories such as `warp-parse` should consume published crates instead of carrying the update implementation sources.

## Migration Scope

Move these directories out of `warp-parse`:

- `crates/wp-update-core`
- `crates/wp-self-update`
- `crates/wp-installer`

Then remove them from the `warp-parse` root workspace.

## Post-Migration Requirements

- `warp-parse` depends on the published `wp-self-update`
- manifest sources are injected by CLI arguments or environment variables, never hard-coded in crates
- each crate has its own README and release notes
- the `wp-update` repository keeps a top-level CHANGELOG for cross-crate release events

## References

- Local reference project: `../wp-lang`
- Chinese counterpart: [../zh/wp_update_repo_layout.md](../zh/wp_update_repo_layout.md)
