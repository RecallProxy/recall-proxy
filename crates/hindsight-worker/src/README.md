# Hindsight Worker Pipeline Scope

`crates/hindsight-worker/src/` is the intended home of background orchestration for hindsight writes.

Primary stages:

1. fetch and normalize transcript artifact,
2. run extraction/fact derivation,
3. execute provider-specific writes,
4. emit completion or failure signals.

Reliability requirements:

- stage-aware retries with exponential backoff,
- idempotent replay behavior through dedupe keys and provider idempotency tokens,
- partial-failure isolation (retry only failed providers).

Canonical behavior is defined in `docs/architecture/hindsight-flow.md`.
