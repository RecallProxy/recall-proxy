# RecallProxy

## Universal Context Gateway for LLMs

RecallProxy is an implementation-agnostic context gateway for LLM applications.
The repository includes a Rust workspace that separates shared contracts, runtime
orchestration, and background processing concerns.

The goal is to decouple agent behavior from memory vendor lock-in so teams can
evolve from simple stores to richer graph or episodic systems without rewriting
agent code.

## Workspace Layout

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

- **recall-proxy-core**: Provider-agnostic memory abstractions (`ContextEngine` trait,
  `MemoryRecord`, `MemoryProvider`), shared domain types (`MemoryQuery`, `ContextSnippet`,
  `MemoryType`), and event delivery contracts.
- **recall-proxy-config**: Serde-serializable configuration for memory routes
  and provider wiring (`GatewayConfig`, `MemoryRouteConfig`, `EngineProviderConfig`).
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

## Memory Provider

RecallProxy ships with a SQLite-backed memory provider as the MVP concrete
implementation. It provides full ingest and query support against a local
SQLite database with zero external server requirements.

### Configuring the SQLite Provider

Add a provider entry to your gateway configuration:

```yaml
providers:
  - id: sqlite:memory.db
    provider_type: semantic
    enabled: true
    capabilities:
      - semantic_search
    settings:
      db_path: memory.db
```

Then create the engine at runtime using `create_provider()`:

```rust
use recall_proxy_config::{GatewayConfig, ProviderConfig, create_provider};
use recall_proxy_core::memory::MemoryProviderKind;

let config = ProviderConfig {
    name: "sqlite:memory.db".to_string(),
    kind: "sqlite".to_string(),
};

let engine = create_provider(&config).await?;
```

### Provider Selection Rationale

SQLite was chosen as the MVP provider because:

- Single-file storage, zero external dependencies beyond the `sqlx` driver
- Full ingest (write) and query (read) support
- No vendor SDK coupling — uses the standard Rust SQLite driver
- Suitable for local development and CI verification
- Can be upgraded to a server-backed provider later with minimal gateway changes

## Project Status

Active foundation stage: core abstractions, orchestration scaffolding, and
test-backed flows are in place. The SQLite memory provider is integrated and
verified with both unit and integration tests.

## License

Distributed under the MIT License.
