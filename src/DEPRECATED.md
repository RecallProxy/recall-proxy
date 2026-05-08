# DEPRECATED

This directory is deprecated and **not part of the Cargo workspace**.

All types, traits, and orchestration logic have been migrated to the `crates/`
workspace. The code here is a stale scaffold that was used during early
prototyping and is no longer compiled or maintained.

## Where to look instead

| Old path in `src/`        | Canonical path in `crates/`        |
|---------------------------|------------------------------------|
| `src/domain/types.rs`     | `crates/core/src/gateway_types.rs` |
| `src/engines/contracts.rs`| `crates/core/src/contracts.rs`     |
| `src/engines/mod.rs`      | `crates/core/src/contracts.rs`     |
| `src/gateway/orchestrator.rs` | `crates/gateway/src/orchestrator.rs` |
| `src/gateway/mod.rs`      | `crates/gateway/src/lib.rs`        |
| `src/lib.rs`              | (no equivalent – workspace entry is `Cargo.toml`) |

When contributing, target the `crates/` hierarchy described in
`docs/architecture/repository-layout.md`.
