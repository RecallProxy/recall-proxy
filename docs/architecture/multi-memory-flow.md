# Multi-Memory Flow

This document describes the full multi-memory flow: **ingest → episodic capture → consolidation → intent-aware retrieval**.

It covers how records flow through the gateway, how the hindsight worker processes them in the background, and how consolidated outputs become retrievable through the gateway.

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
  |--- write to StructuralEngine  (relationship capture)
  |--- write to TemporalEngine    (temporal capture)
  v
IngestReceipt { accepted: true }
  |
  |  HindsightTaskEnqueued (async)
  v
HindsightPipeline
  |
  |--- normalize transcript
  |--- extract derived facts
  |--- fan-out to SemanticEngine (consolidation)
  v
Consolidated records in SemanticEngine
  |
  |  POST /retrieve  {"session_id":"s1","prompt":"ns"}
  v
MCP Server (/retrieve)
  |
  v
ContextMemoryGateway.assemble_context()
  |
  |--- query SemanticEngine   (consolidated/episodic)
  |--- query StructuralEngine (relationships)
  |--- query TemporalEngine   (timeline)
  v
Vec<ContextSnippet> (merged, deduped, budgeted)
```

## Phase 1: Ingest

Raw input arrives at the MCP server via `POST /ingest`. The gateway routes the record to:

- **StructuralEngine** — captures entities and relationships extracted from the content.
- **TemporalEngine** — captures the event as a timeline entry with timestamps.

Both writes happen in parallel via `tokio::join!`. If either fails, the ingest is rejected.

**Crate:** `crates/gateway/src/lib.rs` — `ContextMemoryGateway::ingest()`

## Phase 2: Episodic Capture (Hindsight Pipeline)

After ingest succeeds, the gateway publishes a `HindsightTaskEnqueued` event to a background `tokio::mpsc` queue. The hindsight worker consumes the event and:

1. **Normalize** — canonicalizes roles, trims transport noise, standardizes timestamps.
2. **Extract facts** — derives entities, relations, and summaries from the normalized transcript.
3. **Fan-out writes** — writes derived facts to configured memory providers (semantic, structural, temporal).

This is non-blocking for the caller. The caller already received `IngestReceipt { accepted: true }`.

**Crate:** `crates/hindsight-worker/src/lib.rs` — `HindsightPipeline`, `HindsightExtractor`

## Phase 3: Consolidation

The hindsight pipeline's fan-out writes produce **consolidated records** — enriched memory entries that combine information from multiple sources. These are stored in the SemanticEngine (or any configured semantic-capable provider).

Consolidation enables:
- Cross-referencing between structural and temporal records
- Semantic search over extracted facts
- Long-term memory that persists beyond raw transcript boundaries

**Crate:** `crates/memory/src/engine.rs` — `SqliteMemoryEngine` (or any `ContextEngine` implementation)

## Phase 4: Intent-Aware Retrieval

The client sends `POST /retrieve` with a query prompt. The gateway:

1. **Fan-out queries** — queries all registered engines (semantic, structural, temporal) in parallel.
2. **Merge** — combines results in configured precedence order.
3. **Deduplicate** — removes duplicate snippets by `source_ref` or normalized text.
4. **Budget** — applies token budget limits, dropping snippets that exceed the budget.
5. **Synthesize** — renders prompt-ready context text.

**Crate:** `crates/gateway/src/lib.rs` — `ContextMemoryGateway::assemble_context()`
**Crate:** `crates/gateway/src/context_assembly/mod.rs` — `assemble_context()`

## Engine Types

| Type | Role | Phase |
|------|------|-------|
| **Structural** | Entity/relation graphs | Ingest, Retrieval |
| **Temporal** | Timeline/event capture | Ingest, Retrieval |
| **Semantic** | Embedding/consolidation | Consolidation, Retrieval |

## Provider Abstraction

All engines implement the `ContextEngine` trait from `recall-proxy-core`:

```rust
#[async_trait]
pub trait ContextEngine: Send + Sync {
    fn memory_type(&self) -> MemoryProviderKind;
    async fn write(&self, record: MemoryRecord) -> Result<(), EngineError>;
    async fn query(&self, query: MemoryQuery) -> Result<Vec<ContextSnippet>, EngineError>;
}
```

This allows swapping providers (e.g., SQLite → PostgreSQL, in-memory → Redis) without gateway changes.

## Configuration-Driven Routing

The `RecallProxyConfig` model in `crates/config` defines:

- **providers** — engine inventory with capabilities and settings
- **read_pipelines** — request-time retrieval routing with priority/weight
- **write_pipelines** — response-time and async persistence routing

Engine selection is declarative, not hard-coded. See `docs/architecture/configuration.md`.

## Testing the Flow

See `docs/architecture/mvp-flow.md` for the MVP happy path. Integration tests are in:

- `crates/mcp-server/tests/integration.rs` — gateway-level ingest/retrieve flows
- `crates/memory/tests/integration.rs` — SQLite provider ingest/query flows
- `crates/gateway/src/lib.rs` (tests module) — gateway routing and assembly
- `crates/gateway/src/orchestrator.rs` (tests module) — per-engine-type orchestrator

Run all tests:

```bash
cargo test
```

## Related Documents

- `docs/architecture/request-flow.md` — request-time context assembly pipeline
- `docs/architecture/hindsight-flow.md` — hindsight extraction pipeline
- `docs/architecture/configuration.md` — configuration schema
- `docs/architecture/repository-layout.md` — crate boundaries
- `docs/architecture/mvp-flow.md` — MVP happy-path flow
