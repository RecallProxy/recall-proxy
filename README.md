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

- `recall-proxy-core`: provider-agnostic memory abstractions and shared domain types.
- `recall-proxy-config`: configuration schema used to wire runtime components.
- `recall-proxy-gateway`: HTTP/API-facing orchestration entrypoints.
- `recall-proxy-hindsight-worker`: async background processing boundary for hindsight jobs.

## Public Surface (Initial)

- `recall-proxy-core`
  - `MemoryRecord`
  - `MemoryProvider`
- `recall-proxy-config`
  - `GatewayConfig`
  - `ProviderConfig`
- `recall-proxy-gateway`
  - `GatewayRuntime`
- `recall-proxy-hindsight-worker`
  - `HindsightJob`
  - `WorkerRuntime`

## Design Intent

The workspace is structured so provider implementations can be added as separate
crates (for example, `crates/providers/*`) without coupling SDK-specific code to
the gateway runtime. Runtime crates depend on shared traits from `core` rather
than directly on provider SDKs.

## Hindsight Pipeline Design

The asynchronous response-path design for transcript capture, background handoff, worker stages, and replay-safe retries is documented in:

- `docs/architecture/hindsight-flow.md`

Intended implementation targets are outlined in:

- `crates/gateway/src/response/`
- `crates/core/src/events/`
- `crates/core/src/memory/`
- `crates/hindsight-worker/src/`

## License

## Current Implementation Snapshot

The initial Rust workspace now includes `crates/core/src/memory.rs`, which defines:

- `RawTranscript` for temporal ingest boundaries.
- `DerivedFact` for extracted structural artifacts.
- `ProviderWritePayload` and `ProviderWriteBody` for normalized provider write contracts across semantic, structural, and temporal engines.
Distributed under the MIT License.
