#RecallProxy 🧠

##The Universal Context Gateway for LLMs.

RecallProxy is a high-performance middleware currently in the conceptual and architectural design phase. It sits between your AI Agents and LLM providers to act as a neutral orchestration layer for cognitive memory.

The project’s mission is to decouple agent logic from specific memory implementations. By moving context management into a dedicated gateway, developers can evolve their memory stack—switching from simple vector stores to complex graph structures—without refactoring their application code.

🏗 The Architecture: Decoupled & Agnostic
RecallProxy is designed to be implementation-agnostic. It defines high-level "Memory Types" rather than forcing specific "Memory Brands."

1. Unified Interface
The gateway provides a standardized API for the three pillars of machine memory:

Semantic Memory: Similarity-based retrieval (e.g., Vector DBs).

Structural Memory: Relationship-based retrieval (e.g., Knowledge Graphs).

Temporal/Episodic Memory: Time-ordered conversation history and state.

2. The Orchestration Flow
RecallProxy manages the complex "Write" and "Read" cycles of memory asynchronously to ensure the agent remains fast and responsive. A typical flow looks like:

The Ingest (Write): Raw data is intercepted from an agent interaction. RecallProxy routes this data to a Structural Engine (to map relationships) and simultaneously to a Temporal Engine for long-term archival.

The Hindsight Pattern: Complex extraction (turning raw text into structured memory) happens as a background task. The gateway ensures that today's raw conversation becomes tomorrow's searchable context without blocking the current LLM response.

The Assembly (Read): Before a request is forwarded to the LLM, RecallProxy queries the configured engines in parallel, synthesizes the results, and injects the "perfect" context into the system prompt.

3. Future-Proofing
Start simple by integrating a single engine (like a basic vector store). As your agent's needs grow, you can add or swap implementations—integrating graph engines or specialized episodic databases—by simply updating the RecallProxy configuration.

⚡ Built for Performance (Rust)
RecallProxy is implemented in Rust to ensure that adding a middleware layer doesn't add a latency penalty.

Zero-Cost Abstractions: High-level memory routing with low-level speed.

Async-First: Leveraging Rust's tokio runtime to handle concurrent memory engine lookups and background extraction pipelines efficiently.

🚦 Project Status: Conceptual
RecallProxy is currently being architected. We are focusing on defining the core Memory Traits that will allow any database or memory service to be plugged into the gateway.

Focus Areas:

Designing the ContextEngine trait system.

Developing the async "Hindsight" extraction pipeline.

Creating a standard configuration schema for multi-engine orchestration.

## Configuration-First Orchestration

RecallProxy now defines a provider-based configuration schema in `crates/config/src/lib.rs` with explicit routing fields for:

- request-time reads (`read_pipelines`)
- response-time and asynchronous writes (`write_pipelines`)
- provider-specific settings (`providers[].settings`)
- deterministic multi-provider routing using `priority` + `weight`

To explore configuration evolution paths, see:

- `config/examples/simple-single-engine.yaml`
- `config/examples/multi-engine-orchestration.yaml`
- `docs/architecture/configuration.md`

📜 License
Distributed under the MIT License.

Built with ❤️ for the Agentic future.
