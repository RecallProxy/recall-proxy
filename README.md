# RecallProxy 🧠

## The Universal Context Gateway for LLMs.

RecallProxy is a high-performance middleware currently in the conceptual and architectural design phase. It sits between your AI Agents and LLM providers to act as a neutral orchestration layer for cognitive memory.

The project's mission is to decouple agent logic from specific memory implementations. By moving context management into a dedicated gateway, developers can evolve their memory stack—switching from simple vector stores to complex graph structures—without refactoring their application code.

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
  - `ReadPipeline`
  - `WritePipeline`
- `recall-proxy-gateway`
  - `GatewayRuntime`
  - `response::ChunkCapture`
  - `response::ChunkEvent`
  - `response::FinalizedResponse`
  - `response::FinishReason`
  - `response::NonBlockingHandoffOrchestrator`
- `recall-proxy-hindsight-worker`
  - `HindsightJob`
  - `WorkerRuntime`

## Architecture: Decoupled & Agnostic

RecallProxy is designed to be implementation-agnostic. It defines high-level "Memory Types" rather than forcing specific "Memory Brands."

### 1. Unified Interface

The gateway provides a standardized API for the three pillars of machine memory:

- **Semantic Memory**: Similarity-based retrieval (e.g., Vector DBs).
- **Structural Memory**: Relationship-based retrieval (e.g., Knowledge Graphs).
- **Temporal/Episodic Memory**: Time-ordered conversation history and state.

### 2. The Orchestration Flow

RecallProxy manages the complex "Write" and "Read" cycles of memory asynchronously to ensure the agent remains fast and responsive.

- **The Ingest (Write)**: Raw data is intercepted from an agent interaction. RecallProxy routes this data to a Structural Engine (to map relationships) and simultaneously to a Temporal Engine for long-term archival.
- **The Hindsight Pattern**: Complex extraction (turning raw text into structured memory) happens as a background task. The gateway ensures that today's raw conversation becomes tomorrow's searchable context without blocking the current LLM response.
- **The Assembly (Read)**: Before a request is forwarded to the LLM, RecallProxy queries the configured engines in parallel, synthesizes the results, and injects the "perfect" context into the system prompt.

### 3. Future-Proofing

Start simple by integrating a single engine (like a basic vector store). As your agent's needs grow, you can add or swap implementations—integrating graph engines or specialized episodic databases—by simply updating the RecallProxy configuration.

## Configuration-First Orchestration

RecallProxy defines a provider-based configuration schema in `crates/config/src/lib.rs` with explicit routing fields for:

- request-time reads (`read_pipelines`)
- response-time and asynchronous writes (`write_pipelines`)
- provider-specific settings (`providers[].settings`)
- deterministic multi-provider routing using `priority` + `weight`

To explore configuration evolution paths, see:

- `config/examples/simple-single-engine.yaml`
- `config/examples/multi-engine-orchestration.yaml`
- `docs/architecture/configuration.md`

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

The initial Rust workspace now includes `crates/core/src/memory.rs`, which defines:

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
