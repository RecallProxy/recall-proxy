# Multi-Memory Domain Model

## Goal

Define the canonical data model that RecallProxy uses to represent raw episodes,
durable facts, temporal context, structural links, and retrieval intent. The model
decouples the gateway from any specific memory vendor while providing explicit
distinctions between the four memory categories.

## Canonical Artifact Types

RecallProxy distinguishes four canonical memory artifact types, each with its own
shape and serialization contract:

### Episodic

Raw, unextracted transcript turns captured during a session. These represent the
fidelity layer — the original user/assistant/tool/system utterances before any
hindsight extraction.

```yaml
kind: episodic
session_id: sess-1
turn_id: turn-42
speaker: user
content: "What is the status of order #123?"
observed_at: "2026-05-07T07:00:00Z"
metadata: {}
```

### Semantic

Durable facts extracted from one or more episodes. Each fact is a
subject-predicate-object triple with a confidence score and provenance metadata.

```yaml
kind: semantic
fact_id: fact-7
session_id: sess-1
subject: user
predicate: has_order
object: "order-123"
confidence: 0.95
source_turn_ids:
  - turn-42
extracted_at: "2026-05-07T07:01:00Z"
```

### Temporal

Time-ordered context records (timeline entries, session windows). These enable
time-window queries and chronological reasoning.

```yaml
kind: temporal
session_id: sess-1
window_start: "2026-05-07T06:00:00Z"
window_end: "2026-05-07T08:00:00Z"
content: "User inquired about order status"
metadata:
  context: "support"
```

### Structural

Graph-based relationship records (edges in a knowledge graph). These enable
neighborhood queries and structural reasoning.

```yaml
kind: structural
source_ref: "user:alice"
target_ref: "user:bob"
relation_type: "knows"
weight: 0.8
metadata: {}
```

## Retrieval Intent Taxonomy

The `RetrievalIntent` enum expresses what the caller is asking the gateway to
retrieve. Each variant maps to one or more engine types:

| Intent       | Engine Types                        | Description                                    |
|-------------|--------------------------------------|------------------------------------------------|
| `episodic`  | Temporal                             | Raw transcript turns                           |
| `semantic`  | Semantic                             | Extracted durable facts                        |
| `temporal`  | Temporal                             | Time-window context                            |
| `structural`| Structural                           | Graph relationships                            |
| `mixed`     | Semantic + Structural + Temporal     | Retrieve from all engines and merge results    |

### Default Behavior

When no intent is specified, the gateway defaults to `mixed`, which queries all
registered engines and merges results according to the configured merge policy.

## Query Types

### `MemoryQuery` (legacy gateway)

```rust
pub struct MemoryQuery {
    pub session_id: String,
    pub prompt: String,
    pub max_results: usize,
    pub retrieval_intent: RetrievalIntent,  // defaults to Mixed
}
```

### `ContextRequest` (pipeline gateway)

```rust
pub struct ContextRequest {
    pub tenant_id: String,
    pub agent_id: String,
    pub conversation_id: Option<String>,
    pub user_query: String,
    pub max_context_tokens: usize,
    pub metadata: HashMap<String, String>,
    pub retrieval_intent: RetrievalIntent,  // defaults to Mixed
}
```

## Configuration-Driven Routing

The config schema supports intent-based filtering in read pipelines. A route with
an `intent` filter only serves requests whose intent matches (or is a superset of)
the configured value.

```yaml
read_pipelines:
  - id: episodic_lookup
    providers:
      - provider_id: episodic_store
        capability: episodic_retrieve
        intent: episodic       # only serves episodic intent requests
        priority: 5
```

Routes without an `intent` filter serve all intents (equivalent to `intent: mixed`).

## Provider Payload Extension

The `ProviderWriteBody` enum gained an `episodic` variant to support raw episode
ingestion before hindsight extraction:

```rust
pub enum ProviderWriteBody {
    Episodic { transcript: RawTranscript },
    Temporal { transcript: RawTranscript },
    Structural { facts: Vec<DerivedFact> },
    Semantic { transcript: RawTranscript, facts: Vec<DerivedFact> },
}
```

## Backward Compatibility

### Deprecated types

The following types remain available but are deprecated in favor of the canonical
models:

- `MemoryType` (gateway_types) → use `ContextEngineType` + `RetrievalIntent`
- `MemoryProviderKind` (memory) → use `ContextEngineType`
- `MemoryKind` (memory) → use `MemoryArtifactKind`
- `GatewayConfig` (config) → use `RecallProxyConfig`

Migration helpers are provided:

- `GatewayConfig::to_canonical()` converts legacy config to the canonical model.
- `From<ContextEngineType> for MemoryProviderKind` provides a safe conversion path.

### Migration notes

1. New code should use `RetrievalIntent` and `MemoryArtifact` types directly.
2. Existing integrations using `MemoryQuery` continue to work; the new
   `retrieval_intent` field defaults to `Mixed` for backward compatibility.
3. Config files using the old `GatewayConfig` format are auto-migrated via
   `to_canonical()` at runtime.

## Crate Responsibilities

| Crate | Responsibility |
|-------|---------------|
| `recall-proxy-core` | Canonical domain types: `RetrievalIntent`, `MemoryArtifact`, `MemoryArtifactKind`, `ContextEngineType`, provider traits |
| `recall-proxy-config` | Intent-aware pipeline configuration: `ReadProviderRoute.intent`, provider registration with capabilities |
| `recall-proxy-gateway` | Intent-based routing: selects engines matching the request's `retrieval_intent` |
| `recall-proxy-hindsight-worker` | Converts episodic artifacts into semantic/structural facts |

## Non-Goals

- Concrete storage/provider SDK implementations.
- HTTP endpoint changes beyond what is required by the shared contracts.
- Tokenization or embedding vendor-specific logic.
