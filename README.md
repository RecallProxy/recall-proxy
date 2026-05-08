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
│   ├── hindsight-worker  # Async background extraction pipeline
│   └── mcp-server     # MCP server runtime for RecallProxy
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
- **recall-proxy-mcp-server**: Runnable MCP server that exposes memory ingest
  and query operations over the MCP protocol via JSON-RPC on stdio. Routes
  requests through the `ContextMemoryGateway` with in-memory engine fallbacks.

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

## Running the MCP Server

Start the MCP server:

```bash
cargo run -p recall-proxy-mcp-server
```

The server reads JSON-RPC requests from stdin and writes responses to stdout.
It uses in-memory engines by default (semantic, structural, temporal).

### MCP Protocol Methods

| Method | Description |
|---|---|
| `initialize` | Initialize the MCP session and return server capabilities |
| `initialized` | Notification that the client is fully initialized |
| `memory/ingest` | Ingest a memory record (params: `namespace`, `content`) |
| `memory/query` | Query memory across all engines (params: `session_id`, `prompt`, optional `max_results`) |

### Example: Manual JSON-RPC via stdin

```bash
echo '{"jsonrpc":"2.0","method":"initialize","id":1}' | cargo run -p recall-proxy-mcp-server
```

Expected response:

```json
{"jsonrpc":"2.0","result":{"protocol_version":"2025-03-26","server_info":{"name":"recall-proxy-mcp-server","version":"0.1.0"},"capabilities":{"memory":{"ingest":true,"query":true}}},"id":1}
```

### Example: Ingest and Query

```bash
printf '{"jsonrpc":"2.0","method":"initialize","id":1}\n{"jsonrpc":"2.0","method":"memory/ingest","params":{"namespace":"session-1","content":"user prefers dark mode"},"id":2}\n{"jsonrpc":"2.0","method":"memory/query","params":{"session_id":"session-1","prompt":"preferences"},"id":3}\n' | cargo run -p recall-proxy-mcp-server
```

### Connecting with an MCP Client

Any MCP-compatible client can connect by launching the server as a subprocess with stdio transport. For example, using the MCP Python SDK:

```python
from mcp import ClientSession

async with ClientSession() as session:
    await session.initialize()
    result = await session.call_tool(
        "memory/query",
        {"session_id": "s1", "prompt": "hello"}
    )
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
