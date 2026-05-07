# Repository Layout

This document defines the initial crate boundaries for RecallProxy.

## Workspace Modules

### `crates/core` (`recall-proxy-core`)

**Responsibility:** Provider-agnostic contracts and shared domain types.

**Public surface:**
- `MemoryRecord`
- `MemoryProvider`

This crate must not depend on transport or concrete provider SDK crates.

### `crates/config` (`recall-proxy-config`)

**Responsibility:** Configuration schema shared by runtimes and workers.

**Public surface:**
- `GatewayConfig`
- `ProviderConfig`

This crate stays pure-data to keep config loading/parsing and runtime wiring
decoupled.

### `crates/gateway` (`recall-proxy-gateway`)

**Responsibility:** Request-facing orchestration layer for API traffic.

**Public surface:**
- `GatewayRuntime`

This crate orchestrates read/write flows and depends on `core` traits instead of
provider implementations.

### `crates/hindsight-worker` (`recall-proxy-hindsight-worker`)

**Responsibility:** Background ingestion and extraction pipelines.

**Public surface:**
- `HindsightJob`
- `WorkerRuntime`

This crate isolates asynchronous work from latency-sensitive gateway paths.

## Extension Points

- Provider integrations should live in dedicated crates (for example,
  `crates/providers/<provider-name>`).
- Provider crates implement `MemoryProvider` from `recall-proxy-core`.
- Runtime crates (`gateway`, `hindsight-worker`) consume provider behavior
  through traits and configuration, not SDK-specific APIs.
