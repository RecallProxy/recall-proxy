# Request-Time Context Assembly Pipeline

## Goal

Define the request path from inbound agent call to outbound LLM call, including parallel memory-engine retrieval, merge/synthesis policy, and clear separation between configuration and runtime logic.

## End-to-End Request Flow

1. **Inbound request normalization**
   - API layer receives an agent request and normalizes it into `ContextRequest`.
   - Runtime enriches request metadata (tenant, agent, conversation, tracing IDs).
2. **Budget derivation**
   - Runtime derives `TokenBudget` from configured limits and request overrides.
   - A fixed reserve is held for system instructions and user message.
3. **Parallel context lookup**
   - Orchestrator selects all enabled engines from `ContextPipelineConfig`.
   - Engine calls execute concurrently with:
     - per-engine timeout
     - global timeout cap
     - bounded parallelism (`max_parallel_engines`)
4. **Failure-tolerant join**
   - Lookup errors are captured in `EngineLookupResult.error`.
   - Timeouts are tracked in `EngineLookupMetrics.timed_out`.
   - If `fail_open = true`, orchestration continues with available results.
   - If `fail_open = false` and no engine succeeds, request is rejected early.
5. **Deterministic merge and synthesis**
   - Results are merged using configured `precedence`.
   - Deduplication happens before budgeting.
   - Token truncation is deterministic: snippets are admitted in precedence+rank order until budget exhaustion.
   - Synthesis output is rendered as prompt-ready context text.
6. **Outbound LLM call**
   - Runtime composes final prompt: system prompt + synthesized context + user prompt.
   - Gateway forwards outbound request to configured LLM adapter.

## Parallel Lookup Behavior

- **Fan-out model:** one async task per enabled engine provider.
- **Join model:** gather all task handles; partial completion is allowed.
- **Backpressure:** semaphore limits in-flight lookups to `max_parallel_engines`.
- **Latency accounting:** each provider reports duration in `EngineLookupMetrics.latency`.
- **Cancellation policy:** global timeout cancels unfinished providers; completed results remain valid.

## Merge and Synthesis Rules

### Precedence

- `merge_policy.precedence` is the source of truth for ordering.
- Engines omitted from precedence are appended in stable engine-name order as lowest priority.

### Deduplication

- Strategy selected by `merge_policy.dedupe_strategy`.
- Default behavior:
  - prefer `source_ref` when available
  - otherwise normalize snippet text (trim + lowercase) for keying

### Truncation and Token Budgeting

- Effective context budget:
  - `total_budget`
  - minus `reserved_for_system_prompt`
  - minus `reserved_for_user_prompt`
- Snippets are accepted greedily in deterministic order.
- A snippet that would exceed budget is dropped; orchestration does not partially slice snippets.
- `AssembledContext.dropped_snippets` counts both dedupe drops and budget drops.

### Failure Handling

- Per-engine failures never silently disappear:
  - warning entries emitted in `AssembledContext.warnings`
  - optional telemetry event per failure class (timeout, transport, parse)
- If all engines fail:
  - `fail_open = true` => continue with empty context and warning
  - `fail_open = false` => propagate orchestration error to caller

## Configuration vs Runtime Responsibilities

## Configuration (`crates/config`)

- Engine inventory (enabled/disabled, type, timeout, weighting).
- Concurrency and timeout limits.
- Merge policy (precedence, dedupe strategy, max snippets).
- Budget defaults and required reserves.

## Runtime Orchestration (`crates/gateway`)

- Request normalization and metadata binding.
- Async fan-out/fan-in execution.
- Error capture and policy-driven fail-open/fail-closed behavior.
- Deterministic merge and context synthesis.
- Prompt composition and hand-off to LLM transport.

## Shared Domain (`crates/core`)

- Typed request/response payloads for context lookup.
- Engine result and metrics contracts.
- Assembled context shape used by gateway and API boundaries.

## Non-Goals

- No production HTTP handler implementation.
- No vendor SDK-specific tokenization/counting.
- No retrieval heuristics tied to a specific memory vendor.
