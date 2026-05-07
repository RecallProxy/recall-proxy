# Configuration Schema

RecallProxy configuration is provider-driven. Runtime behavior is determined by declarative routing rules instead of hard-coded adapters.

## Top-Level Model

- `providers`: Registry of all memory engines available to the gateway.
- `read_pipelines`: Request-time retrieval orchestration.
- `write_pipelines`: Request/response/background persistence orchestration.

## Provider Registration

Each provider includes:

- `id`: Stable reference used by routes.
- `provider_type`: High-level class (`semantic`, `structural`, `temporal`, `composite`).
- `enabled`: Feature toggle at provider level.
- `capabilities`: Explicit list of supported operations.
- `settings`: Provider-specific key/value options (endpoint, index, namespace, stream, etc.).

`settings` intentionally stays unopinionated so teams can add engine-specific parameters without schema churn.

## Request-Time Reads (`read_pipelines`)

A read pipeline maps capabilities to one or more providers:

- `provider_id`
- `capability`
- `priority` (lower value means higher priority)
- `weight` (tie-breaker when priorities match; higher wins)
- `enabled`

### Routing Semantics

When multiple providers serve the same capability:

1. Filter to enabled routes with matching capability.
2. Sort ascending by `priority`.
3. For equal priority, sort descending by `weight`.
4. If still tied, use deterministic provider ID ordering.

This gives predictable failover and weighted preference without runtime code changes.

## Response-Time and Async Writes (`write_pipelines`)

Write pipelines declare `trigger` plus destination sinks:

- `trigger`: `on_request`, `on_response`, `async_hindsight`
- `sinks[]` entries:
  - `provider_id`
  - `capability`
  - `enabled`
  - `criticality` (`required` or `best_effort`)

`required` means a sink failure should fail the write stage; `best_effort` means the gateway logs and continues.

## Migration Path

- Start simple with one semantic provider and one response write pipeline.
- Add temporal capture (`on_request` + `on_response`) for timeline and session durability.
- Add structural extraction (`async_hindsight`) when relationship memory is needed.
- Introduce fallback providers by adding routes with lower precedence (higher `priority` value).

Reference examples:

- `config/examples/simple-single-engine.yaml`
- `config/examples/multi-engine-orchestration.yaml`
