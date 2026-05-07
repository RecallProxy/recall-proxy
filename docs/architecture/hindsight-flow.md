# Hindsight Async Write Pipeline

This document defines the response-path pipeline used by RecallProxy to:

1. stream LLM output to the caller with minimal latency,
2. persist a raw transcript boundary that can be replayed safely, and
3. hand off memory enrichment to a background worker without blocking the request.

## 1) End-to-End Flow

1. **Request accepted by gateway**
   - Gateway allocates `conversation_id`, `request_id`, and a future `response_id`.
   - Gateway records provider routing metadata (provider, model, region, account/project).

2. **Provider response is streamed**
   - Gateway forwards chunks/tokens to caller immediately.
   - Gateway mirrors streamed chunks into an in-memory response accumulator.
   - Accumulator captures boundaries: first chunk timestamp, last chunk timestamp, finish reason, token counters.

3. **Response finalized**
   - On stream completion (success or interruption), gateway emits one immutable **Raw Transcript Artifact**.
   - Artifact includes request envelope, provider metadata, chunk-derived final text, and boundary timestamps.

4. **Background handoff**
   - Gateway publishes a `HindsightTaskEnqueued` event carrying identifiers + idempotency key + transcript locator.
   - Queue publish is non-blocking for the caller path; caller response already completed.

5. **Worker pipeline**
   - Worker fetches transcript artifact.
   - Stages run in order: normalization -> extraction/fact derivation -> provider-specific writes.
   - Worker emits terminal status event: `completed` or `failed`.

## 2) Gateway Emission Contract

At handoff time, the gateway **must emit**:

- **Identity**
  - `workspace_id`
  - `tenant_id` (if multi-tenant boundary differs from workspace)
  - `conversation_id`
  - `request_id`
  - `response_id`
- **Provider metadata**
  - `provider_name`
  - `model_id`
  - `provider_request_id` (if available)
  - `routing_hint` (region/project/account)
- **Transcript boundaries**
  - `request_received_at`
  - `response_stream_started_at`
  - `response_stream_completed_at`
  - `response_finish_reason`
  - `input_token_count` and `output_token_count` (optional, nullable)
- **Delivery safety**
  - `dedupe_key` (deterministic key derived from `workspace_id + response_id + transcript_checksum`)
  - `attempt` (initially `1`; incremented only on enqueue retry)
  - `trace_id` and `correlation_id` for observability across gateway/worker
- **Payload locator**
  - `transcript_artifact_uri` (object store or durable blob key)
  - `transcript_checksum` (sha256)

## 3) Artifact Boundaries

### Raw transcript artifact

Immutable record of the request/response boundary. Stored once and never mutated.

Contains:
- request messages as received,
- final response text reconstructed from streamed chunks,
- chunk metadata and stream boundaries,
- provider metadata.

### Derived facts artifact

Structured facts produced from the normalized transcript. May evolve across extractor versions.

Contains:
- extracted entities/relations/summaries,
- extraction model/version metadata,
- confidence/sourcing annotations,
- links back to `transcript_checksum` and source spans.

### Provider write payload

Provider-specific transformation of derived facts for a concrete memory backend.

Contains:
- provider-native schema payloads,
- provider idempotency token,
- provenance back to `response_id` and extraction version.

## 4) Worker Stages

1. **Normalize transcript**
   - Canonicalize roles, trim transport noise, standardize timestamps.
   - Output is deterministic to guarantee replay stability.

2. **Extract facts**
   - Run extraction strategy over normalized transcript.
   - Emit derived facts artifact and extraction metadata.

3. **Provider-specific writes**
   - Fan out to configured memory providers (semantic, structural, temporal, etc.).
   - Track per-provider success/failure independently.

4. **Signal completion**
   - Emit completion event when all required providers succeed.
   - Emit failure event with stage + error classification when terminally failed.

## 5) Retry, Failure, and Replay Semantics

### Enqueue failures (gateway side)

- If enqueue fails before response completion: return an error only when policy requires strict durability.
- If enqueue fails after response completion: persist a local outbox record and retry asynchronously.
- Outbox uses the same `dedupe_key` to avoid duplicate worker execution.

### Worker retries

- Retries are stage-aware with exponential backoff + jitter.
- Retries must be idempotent by using:
  - task-level `dedupe_key`,
  - provider-level idempotency token,
  - transcript checksum verification.
- Non-retryable errors are classified terminal and emitted immediately.

### Partial provider-write failures

- Worker records per-provider status.
- Success for provider A does not rollback provider B.
- Failed providers are retried independently; already successful providers are skipped by idempotency token.

### Replay safety

- Replaying the same raw transcript must not create duplicate memory records.
- Reprocessing newer extraction versions is allowed by versioned derived-fact identifiers.
- All outputs keep provenance pointers to original `response_id` and `transcript_checksum`.

## 6) Contract Ownership and Intended Paths

- `crates/core/src/events/`: event/task envelopes, IDs, dedupe metadata
- `crates/core/src/memory/`: transcript, derived-fact, provider-write contracts
- `crates/gateway/src/response/`: streaming capture + handoff emitter orchestration
- `crates/hindsight-worker/src/`: stage execution, retries, provider fanout, completion/failure signalling

These paths are treated as the intended implementation targets even though the repository is currently in conceptual phase.
