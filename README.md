# RecallProxy

## Universal Context Gateway for LLMs

RecallProxy is an implementation-agnostic context gateway for LLM applications.
The repository includes a Rust workspace that separates shared contracts, runtime
orchestration, and background processing concerns.

The goal is to decouple agent behavior from memory vendor lock-in so teams can
evolve from simple stores to richer graph or episodic systems without rewriting
agent code.

## Workspace Layout

> **Note:** The root `src/` directory is deprecated. All active development
> targets the `crates/` workspace hierarchy. See `src/DEPRECATED.md` for
> migration guidance.

```text
.
├── Cargo.toml
├── crates
│   ├── config         # Configuration schema
│   ├── core           # Provider abstractions and domain types
│   ├── gateway        # Ingest routing and context assembly
│   └── hindsight-worker  # Async background extraction pipeline
└── docs
    └── architecture
```

## Crate Responsibilities

- **recall-proxy-core**: Unified `ContextEngine` trait and `ContextEngineType`
  enum (Structural, Temporal, Semantic, Graph). Provider-facing traits
  (`MemoryProvider`, `SemanticMemoryProvider`, etc.), shared domain types
  (`ContextRequest`, `ContextSnippet`, `EngineLookupResult`), event delivery
  contracts, and memory artifact types (`MemoryRecord`, `RawTranscript`,
  `DerivedFact`).
- **recall-proxy-config**: `RecallProxyConfig` — the canonical provider-driven
  config model matching the YAML examples (providers, read_pipelines,
  write_pipelines). Also includes `ContextPipelineConfig` for context assembly
  pipeline settings. Legacy `GatewayConfig` / `ProviderConfig` types are
  deprecated with a migration path via `to_canonical()`.
- **recall-proxy-gateway**: `ContextGateway` (per-engine-type orchestration) and
  `ContextMemoryGateway` (unified `ContextEngine` trait orchestration) with ingest
  routing and context assembly.
- **recall-proxy-hindsight-worker**: `HindsightPipeline` with a background
  `tokio::mpsc` queue and a pluggable `HindsightExtractor` trait.

## Architecture Milestones (Initial Implementation)

1. **ContextEngine trait system** (`crates/core/src/`)
   - Async trait for engine providers with typed `write` and `query` operations.
   - Engine-neutral `MemoryType` enum for semantic, structural, and temporal memory.

2. **Async hindsight extraction pipeline** (`crates/hindsight-worker/src/`)
   - Background queue with `tokio::mpsc`.
   - Pluggable `HindsightExtractor` trait for converting raw interactions into
     structural and temporal records without blocking request flow.

3. **Multi-engine orchestration config schema** (`crates/config/src/`)
   - Serde-serializable configuration for memory routes and provider wiring.
   - Enables provider swapping via configuration, not gateway refactors.

4. **Gateway orchestration** (`crates/gateway/src/`)
   - Ingest routing for structural + temporal writes.
   - Read assembly across registered engine providers.
   - Two complementary orchestrators: `ContextGateway` (per-engine-type traits)
     and `ContextMemoryGateway` (unified `ContextEngine` trait).

## Hindsight Pipeline Design

The asynchronous response-path design for transcript capture, background handoff,
worker stages, and replay-safe retries is documented in:

- `docs/architecture/hindsight-flow.md`

## Running the Gateway Server

Start the gateway with:

```bash
cargo run -p recall-proxy-gateway
```

The server binds to `127.0.0.1:8080` by default. Override with the `RECALL_PROXY_BIND_ADDRESS` environment variable:

```bash
RECALL_PROXY_BIND_ADDRESS=0.0.0.0:3000 cargo run -p recall-proxy-gateway
```

Verify the service is running:

```bash
curl http://127.0.0.1:8080/health
# {"status":"ok","service":"recall-proxy-gateway"}
```

## Docker

You can build and run the gateway server using Docker for local development.

### Build

```bash
docker build -t recall-proxy-gateway .
```

### Run

```bash
docker run -p 8080:8080 recall-proxy-gateway
```

By default, the container binds to `0.0.0.0:8080`. You can customize this using environment variables:

```bash
docker run -p 3000:3000 -e RECALL_PROXY_BIND_ADDRESS=0.0.0.0:3000 recall-proxy-gateway
```

## Testing

Main flows are covered with unit tests:

```bash
cargo test
```

- Ingest writes are routed to structural and temporal engines.
- Read assembly combines context from configured engines.
- Hindsight pipeline processes interactions in the background.

## End-to-End Testing

The repository includes a Docker Compose-based E2E testing setup with promptfoo for validating the gateway's HTTP surface.

### Prerequisites

- Docker and Docker Compose
- promptfoo (`npm install -g promptfoo`)

### Running E2E Tests

Start the local environment:

```bash
cd examples/e2e
docker-compose up --build
```

This starts:
- **gateway**: The RecallProxy gateway service on port 8080
- **mock-hindsight**: A mock hindsight provider on port 8081

Run the promptfoo tests:

```bash
promptfoo eval
```

### Available Endpoints

- `GET /health` - Health check endpoint
- `POST /ingest` - Ingest interaction data

Example ingest request:

```bash
curl -X POST http://localhost:8080/ingest \
  -H "Content-Type: application/json" \
  -d '{"interaction_id": "test-001", "content": "Test interaction"}'
```

## Project Status

Active foundation stage: core abstractions, orchestration scaffolding, and
test-backed flows are in place. The provider config model and engine type
abstractions have been unified across the workspace. Next iterations can add
concrete provider adapters and integration tests against simulated endpoints.

## License

Distributed under the MIT License.
