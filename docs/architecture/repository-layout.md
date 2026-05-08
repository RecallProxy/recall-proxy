# Repository Layout

This document defines the crate boundaries for RecallProxy.

## Workspace Modules

### `crates/core` (`recall-proxy-core`)

**Responsibility:** Provider-agnostic contracts and shared domain types.

**Public surface:**
- `ContextEngineType` — canonical engine type enum (Structural, Temporal, Semantic, Graph)
- `ContextEngine` trait — unified async trait for engine providers
- `ContextRequest`, `ContextSnippet`, `EngineLookupResult` — shared domain types
- `MemoryRecord`, `RawTranscript`, `DerivedFact` — memory artifact types
- `MemoryProvider`, `SemanticMemoryProvider`, `StructuralMemoryProvider`, `TemporalMemoryProvider` — provider-facing traits
- `ProviderError`, `ProviderResult` — error types
- `HandoffEnvelope`, `TraceContext`, `EventId` — event delivery contracts

This crate must not depend on transport or concrete provider SDK crates.

### `crates/config` (`recall-proxy-config`)

**Responsibility:** Configuration schema shared by runtimes and workers.

**Public surface:**
- `RecallProxyConfig` — top-level config matching the YAML examples (providers, read_pipelines, write_pipelines)
- `ProviderRegistration` — provider registry entry
- `ReadPipeline`, `ReadProviderRoute` — request-time routing
- `WritePipeline`, `WriteSink`, `WriteTrigger`, `WriteCriticality` — response/async writes
- `ContextPipelineConfig`, `EngineConfig`, `MergePolicyConfig` — context assembly pipeline config

This crate stays pure-data to keep config loading/parsing and runtime wiring
decoupled.

### `crates/gateway` (`recall-proxy-gateway`)

**Responsibility:** Request-facing orchestration layer for API traffic.

**Public surface:**
- `ContextGateway` — per-engine-type orchestrator (StructuralEngine, TemporalEngine, SemanticEngine)
- `ContextMemoryGateway` — unified ContextEngine trait orchestrator
- `ContextEngineProvider`, `ContextAssembler` — request-path traits
- `RequestContextOrchestrator` — request-time context assembly
- `ChunkCapture`, `FinalizedResponse`, `NonBlockingHandoffOrchestrator` — response pipeline

This crate orchestrates read/write flows and depends on `core` traits instead of
provider implementations.

### `crates/hindsight-worker` (`recall-proxy-hindsight-worker`)

**Responsibility:** Background ingestion and extraction pipelines.

**Public surface:**
- `HindsightPipeline` — background extraction pipeline
- `HindsightExtractor` — pluggable extraction trait
- `HindsightJob`, `WorkerRuntime` — worker runtime boundary

This crate isolates asynchronous work from latency-sensitive gateway paths.

### `crates/mcp-server` (`recall-proxy-mcp-server`)

**Responsibility:** MCP server binary and in-memory engine providers for the MVP path.

**Public surface:**
- `InMemoryContextEngine` — in-memory `ContextEngine` implementation
- `build_router` — constructs the axum `Router` with `/ingest`, `/retrieve`, `/health`
- `default_gateway` — creates a gateway pre-wired with in-memory engines

This crate is the entry point for proving the end-to-end MVP flow.

## Extension Points

- Provider integrations should live in dedicated crates (for example,
  `crates/providers/<provider-name>`).
- Provider crates implement `MemoryProvider` from `recall-proxy-core`.
- Runtime crates (`gateway`, `hindsight-worker`) consume provider behavior
  through traits and configuration, not SDK-specific APIs.
