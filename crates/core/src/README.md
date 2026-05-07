# Core Crate Layout

This module scaffold captures the initial contract for RecallProxy's pluggable
memory gateway. It is intentionally lightweight while the full Cargo workspace
and concrete providers are still being bootstrapped.

## Modules

- `memory/`: provider traits and capability contracts
- `context/`: shared ingest and query request/response models
- `error/`: provider-facing error taxonomy and retryability hints

## Notes

- The traits are async-first and `Send + Sync` to support concurrent fan-out.
- Orchestration concerns (aggregation, fallback, re-ranking) are intentionally
  left to RecallProxy orchestration layers, not provider implementations.
- Type names can evolve as the workspace matures; this scaffold exists to keep
  contracts explicit and testable from day one.
