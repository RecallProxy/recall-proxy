# Memory Contract Surface

`crates/core/src/memory/` owns cross-runtime memory contracts.

The hindsight pipeline separates artifacts into:

- **Raw transcript artifact**: immutable request/response boundary record.
- **Derived facts artifact**: extractor output with versioned provenance.
- **Provider write payload**: provider-specific transformed writes with idempotency tokens.

These contracts are intentionally decoupled so provider adapters can evolve without changing gateway capture semantics.

See `docs/architecture/hindsight-flow.md` for canonical field requirements and retry/replay constraints.
