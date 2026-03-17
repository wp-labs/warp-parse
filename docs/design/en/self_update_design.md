# Self-Update Design

- Status: Draft
- Scope: `wproj self update`

## Background

Warp Parse ships multiple binaries. The update mechanism should upgrade the whole toolset in a controlled and auditable way without pushing replacement risk into the main runtime path.

## Goals

- one update entry for all binaries
- manual check and manual update
- optional automatic check and later optional automatic apply
- version and hash verification
- rollback on installation failure

## Non-Goals

- incremental patching in the first version
- GUI workflows
- complex multi-manifest compatibility layers

## Ownership

- `wproj`: the only binary that executes update actions
- `wparse`: may show update availability later, but should not replace binaries
- `wpgen` and `wprescue`: do not own update flow

## CLI Shape

Planned commands:

```bash
wproj self status
wproj self check
wproj self update
wproj self rollback
wproj self auto enable|disable|set
```

## Local State

Suggested path:

```text
~/.warp_parse/update/
```

Suggested files:

- `policy.toml`
- `state.json`
- `lock`
- `backups/`

## Channel Model

The channel mapping must stay aligned with release branches:

- `stable` <- `main`
- `beta` <- `beta`
- `alpha` <- `alpha`

Cross-channel upgrades must require explicit operator intent.

## Manifest Model

The client should fetch:

```text
updates/<channel>/manifest.json
```

The manifest must include:

- version
- channel
- publish metadata
- platform assets
- sha256 checksums

## Update Flow

1. read current version
2. fetch manifest for the selected channel
3. compare versions
4. download asset and verify hash
5. unpack to a temporary directory
6. acquire update lock
7. create backup
8. replace binaries atomically
9. run health checks
10. persist success or rollback on failure

## Safety Rules

- keep channel isolation strict
- allow downloads only from approved origins
- use file locking
- keep backups for rollback
- write auditable state and failure reasons

## Package Manager Compatibility

If the installation came from a system package manager:

- `check` may still work
- `update` should refuse by default
- `--force` can exist, but must be explicit

## MVP

First implementation:

- `status`, `check`, `update`, `rollback`
- manifest retrieval
- hash verification
- full-package install
- backup and rollback
- local policy and state persistence

Follow-up:

- full `auto` policy management
- update-available hints in runtime tools
- better error codes and observability

## Acceptance Criteria

- all binaries land on the same version after success
- failed replacement rolls back automatically
- concurrent updates do not corrupt the installation
- channel mapping is enforced consistently

## Chinese Counterpart

- [../zh-CN/self_update_design.md](../zh-CN/self_update_design.md)
