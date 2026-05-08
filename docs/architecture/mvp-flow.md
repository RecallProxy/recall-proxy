# MVP Happy-Path Flow

This document traces the end-to-end flow from ingest to retrieval through the
MCP server, using in-memory engine providers.

## Flow Diagram

```
Client
  |
  |  POST /ingest  {"session_id":"s1","namespace":"ns","content":"data"}
  v
MCP Server (/ingest)
  |
  v
ContextMemoryGateway.ingest()
  |
  |--- write to StructuralEngine (in-memory)
  |--- write to TemporalEngine (in-memory)
  v
IngestReceipt { accepted: true }
  |
  |  POST /retrieve  {"session_id":"s1","prompt":"ns"}
  v
MCP Server (/retrieve)
  |
  v
ContextMemoryGateway.assemble_context()
  |
  |--- query SemanticEngine
  |--- query StructuralEngine
  |--- query TemporalEngine
  v
Vec<ContextSnippet> (merged from all engines)
```

## Step-by-Step

### 1. Start the MCP Server

```bash
cargo run -p recall-proxy-mcp-server
```

This starts an axum HTTP server on `127.0.0.1:8081` with three routes:
- `GET /health` — liveness check
- `POST /ingest` — write a `MemoryRecord` to structural and temporal engines
- `POST /retrieve` — query all engines and merge results

The server initializes with three `InMemoryContextEngine` instances (semantic,
structural, temporal) via `default_gateway()`.

### 2. Ingest

```bash
curl -X POST http://127.0.0.1:8081/ingest \
  -H "Content-Type: application/json" \
  -d '{"session_id":"s1","namespace":"ns","content":"data"}'
```

The `/ingest` handler:
1. Deserializes the request body into `IngestRequest`.
2. Wraps it in a `MemoryRecord`.
3. Writes to the structural and temporal engines in parallel via
   `ContextMemoryGateway.ingest()`.
4. Returns `{"status":"accepted"}` on success.

### 3. Retrieve

```bash
curl -X POST http://127.0.0.1:8081/retrieve \
  -H "Content-Type: application/json" \
  -d '{"session_id":"s1","prompt":"ns","max_results":10}'
```

The `/retrieve` handler:
1. Deserializes the request body into `RetrieveRequest`.
2. Converts it to a `MemoryQuery`.
3. Calls `ContextMemoryGateway.assemble_context()` which queries all three
   engine types (semantic, structural, temporal) and merges results.
4. Returns `{"snippets":[...],"count":N}`.

### 4. Verify

```bash
curl http://127.0.0.1:8081/health
# ok
```

## Running the Smoke Tests

```bash
cargo test --test integration
```

The integration test suite covers:
- Gateway ingest routing to structural and temporal engines
- Context assembly combining results from all engines
- Missing engine error handling
- In-memory engine storage and retrieval
- Max results enforcement
- Configuration parsing
