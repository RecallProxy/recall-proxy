# Events Contract Surface

`crates/core/src/events/` owns shared event envelopes passed between runtime boundaries.

For the hindsight pipeline, this path owns:

- task and event identifiers (`conversation_id`, `request_id`, `response_id`),
- gateway-to-worker handoff envelope fields,
- deduplication and idempotency metadata (`dedupe_key`, `attempt`),
- delivery tracing metadata (`trace_id`, `correlation_id`).

See `docs/architecture/hindsight-flow.md` for end-to-end semantics.
