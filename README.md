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
│   ├── hindsight-worker  # Async background extraction pipeline
│   └── mcp-server     # MCP server with in-memory engines for the MVP path
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
- **recall-proxy-mcp-server**: HTTP MCP server exposing `/ingest` and `/retrieve`
  endpoints backed by in-memory `ContextEngine` providers. Used for MVP
  verification and local development.

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

## MVP Happy Path: Ingest to Retrieval

The MVP path uses in-memory engines behind the MCP server to prove the
end-to-end flow without external dependencies.

1. **Start the MCP server**

   ```bash
   cargo run -p recall-proxy-mcp-server
   ```

   The server binds to `127.0.0.1:8081` by default.

2. **Ingest a record**

   ```bash
   curl -X POST http://127.0.0.1:8081/ingest \
     -H "Content-Type: application/json" \
     -d '{
       "session_id": "demo-session",
       "namespace": "user-preferences",
       "content": "user prefers Rust"
     }'
   ```

3. **Retrieve context**

   ```bash
   curl -X POST http://127.0.0.1:8081/retrieve \
     -H "Content-Type: application/json" \
     -d '{
       "session_id": "demo-session",
       "prompt": "user-preferences",
       "max_results": 10
     }'
   ```

4. **Verify health**

   ```bash
   curl http://127.0.0.1:8081/health
   # ok
   ```

Detailed architecture docs:

- `docs/architecture/request-flow.md` — request-time context assembly pipeline
- `docs/architecture/configuration.md` — configuration schema
- `docs/architecture/repository-layout.md` — crate boundaries
- `docs/architecture/hindsight-flow.md` — hindsight extraction pipeline

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

Main flows are covered with unit and integration tests:

```bash
cargo test
```

- Ingest writes are routed to structural and temporal engines.
- Read assembly combines context from configured engines.
- Hindsight pipeline processes interactions in the background.
- End-to-end MVP path (ingest -> retrieve) verified via `tests/integration.rs`.

## Project Status

Shipped capabilities:

- `recall-proxy-core` — `ContextEngine` trait, provider abstractions, shared domain types
- `recall-proxy-config` — `GatewayConfig` / `ProviderConfig` schema
- `recall-proxy-gateway` — `ContextMemoryGateway` with ingest routing and context assembly
- `recall-proxy-mcp-server` — HTTP server (`/ingest`, `/retrieve`, `/health`) backed by in-memory engines
- `tests/integration.rs` — end-to-end smoke tests covering the MVP ingest-to-retrieve path
- `docs/architecture/` — request-flow, configuration, repository-layout, and hindsight-flow docs

Next iterations: concrete provider adapters, integration tests against simulated endpoints,
and production-ready backend stores.

## License

Distributed under the MIT License.
