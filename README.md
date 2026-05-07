# RecallProxy

RecallProxy is an implementation-agnostic context gateway for LLM applications.
The repository now includes a bootstrap Rust workspace that separates shared
contracts, runtime orchestration, and background processing concerns.

## Workspace Layout

```text
.
├── Cargo.toml
├── crates
│   ├── config
│   ├── core
│   ├── gateway
│   └── hindsight-worker
└── docs
    └── architecture
        └── repository-layout.md
```

## Crate Responsibilities

- `recall-proxy-core`: provider-agnostic memory abstractions, shared domain types, and engine contracts.
- `recall-proxy-config`: configuration schema used to wire runtime components.
- `recall-proxy-gateway`: HTTP/API-facing orchestration entrypoints with async ingest and context assembly flows.
- `recall-proxy-hindsight-worker`: async background processing boundary for hindsight jobs.

## Public Surface (Initial)

- `recall-proxy-core`
  - `MemoryRecord`
  - `MemoryProvider`
  - `StructuralEngine`, `TemporalEngine`, `SemanticEngine`, `HindsightProcessor` (async trait contracts)
- `recall-proxy-config`
  - `GatewayConfig`
  - `ProviderConfig`
- `recall-proxy-gateway`
  - `GatewayRuntime`
<<<<<<< HEAD
  - `response::ChunkCapture`
  - `response::ChunkEvent`
  - `response::FinalizedResponse`
  - `response::FinishReason`
  - `response::NonBlockingHandoffOrchestrator`
=======
  - `ContextGateway` (async orchestrator with parallel ingest/context assembly)
>>>>>>> 4d71284 (Implement crate scaffolding: domain types, engine contracts, and async gateway orchestrator)
- `recall-proxy-hindsight-worker`
  - `HindsightJob`
  - `WorkerRuntime`

## Design Intent

The workspace is structured so provider implementations can be added as separate
crates (for example, `crates/providers/*`) without coupling SDK-specific code to
the gateway runtime. Runtime crates depend on shared traits from `core` rather
than directly on provider SDKs.

## Hindsight Pipeline Design

The asynchronous response-path design for transcript capture, background handoff,
worker stages, and replay-safe retries is documented in:

- `docs/architecture/hindsight-flow.md`

Intended implementation targets are outlined in:

- `crates/gateway/src/response/`
- `crates/core/src/events/`
- `crates/core/src/memory/`
- `crates/hindsight-worker/src/`

## Current Implementation Snapshot

The initial Rust workspace includes:
- `crates/core/src/memory.rs` — `RawTranscript`, `DerivedFact`, `ProviderWritePayload`, `ProviderWriteBody`
- `crates/core/src/contracts.rs` — async trait contracts for Structural, Temporal, Semantic, and Hindsight engines
- `crates/core/src/gateway_types.rs` — `MemoryPayload`, `MemoryQuery`, `ContextSnippet`, `IngestReceipt`, `EngineError`, `MemoryType`
- `crates/gateway/src/orchestrator.rs` — async `ContextGateway` with parallel ingest routing and context assembly, including unit tests

The scaffold is provider-neutral: concrete engine adapters can be added without changing gateway-level contracts.

## License

- `RawTranscript` for temporal ingest boundaries.
- `DerivedFact` for extracted structural artifacts.
- `ProviderWritePayload` and `ProviderWriteBody` for normalized provider write
  contracts across semantic, structural, and temporal engines.

The gateway response streaming capture contract in `crates/gateway/src/response`
supports:

- Chunk capture with sequence validation.
- Response finalization metadata (finish reason, start and completion times).
- Non-blocking handoff orchestration for background memory ingestion pipelines.

## License

Distributed under the MIT License.
