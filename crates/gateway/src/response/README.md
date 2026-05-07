# Gateway Response Pipeline Scope

`crates/gateway/src/response/` is the intended home of:

1. streamed provider chunk capture,
2. transcript boundary finalization, and
3. non-blocking handoff enqueue to hindsight worker tasks.

The gateway path must guarantee:

- caller streaming is never blocked by extraction/write work,
- finalized transcript artifacts are immutable and checksummed,
- handoff events include identity, provider metadata, boundaries, and dedupe keys.

Design details are specified in `docs/architecture/hindsight-flow.md`.
