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
│   ├── mcp-server     # MCP server with in-memory engines for the MVP path
│   └── memory         # SQLite-backed memory provider (ContextEngine impl)
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
- **recall-proxy-memory**: SQLite-backed `ContextEngine` implementation.
  Provides a concrete, file-based memory engine for local development and CI
  verification. Implements `ContextEngine` with full ingest and query support.

## Architecture Milestones (Initial Implementation)

1. **ContextEngine trait system** (`crates/core/src/`)
    - Async trait for engine providers with typed `write` and `query` operations.
    - Engine-neutral `ContextEngineType` enum for semantic, structural, temporal, and graph memory.

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

5. **SQLite-backed memory provider** (`crates/memory/src/`)
    - Concrete `ContextEngine` implementation with real persistence.
    - Provider factory for creating engines from `ProviderRegistration` config.
    - Suitable for local development and CI verification.

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

## Multi-Memory Flow

The full multi-memory flow traces: **ingest → episodic capture → consolidation → intent-aware retrieval**.

1. **Ingest** — `POST /ingest` writes to structural and temporal engines in parallel.
2. **Episodic capture** — A `HindsightTaskEnqueued` event triggers the hindsight pipeline, which normalizes, extracts facts, and writes consolidated records to the semantic engine.
3. **Consolidation** — Derived facts from the hindsight worker become available in the semantic engine.
4. **Intent-aware retrieval** — `POST /retrieve` queries all three engines (semantic, structural, temporal), merges results by precedence, deduplicates, applies token budget, and renders prompt-ready context.

See `docs/architecture/multi-memory-flow.md` for the full architectural description.

## Rollout Plan

### Phase 1: MVP Verification (Current)
- In-memory engines behind the MCP server for local development.
- SQLite-backed engine in `crates/memory` for real persistence without external dependencies.
- Integration tests covering the ingest → retrieve path.

### Phase 2: Provider Expansion
- Add concrete provider adapters (e.g., PostgreSQL, Redis, vector databases).
- Each adapter implements `ContextEngine` trait from `recall-proxy-core`.
- Configuration-driven provider selection via `RecallProxyConfig`.

### Phase 3: Production Readiness
- Hindsight pipeline with durable outbox and retry semantics.
- Streaming response capture with non-blocking handoff.
- Token budgeting and deterministic merge/synthesis rules.

## Exercising the Flow Locally

### 1. Start the MCP Server

```bash
cargo run -p recall-proxy-mcp-server
```

### 2. Ingest a Record

```bash
curl -X POST http://127.0.0.1:8081/ingest \
  -H "Content-Type: application/json" \
  -d '{
    "session_id": "demo-session",
    "namespace": "user-preferences",
    "content": "user prefers Rust"
  }'
```

### 3. Retrieve Context

```bash
curl -X POST http://127.0.0.1:8081/retrieve \
  -H "Content-Type: application/json" \
  -d '{
    "session_id": "demo-session",
    "prompt": "user-preferences",
    "max_results": 10
  }'
```

### 4. Verify Health

```bash
curl http://127.0.0.1:8081/health
# ok
```

### 5. Run Integration Tests

```bash
cargo test
```

This exercises:
- Gateway ingest routing to structural and temporal engines
- Context assembly combining results from all engines
- SQLite provider ingest and query flows
- Missing engine error handling
- Configuration parsing

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
- `recall-proxy-memory` — SQLite-backed `ContextEngine` implementation with provider factory
- `tests/integration.rs` — end-to-end smoke tests covering the MVP ingest-to-retrieve path
- `docs/architecture/` — request-flow, configuration, repository-layout, hindsight-flow, and multi-memory-flow docs

Next iterations: concrete provider adapters, integration tests against simulated endpoints,
and production-ready backend stores.

## License

Distributed under the MIT License.
