RecallProxy 🧠
The Universal Context Gateway for LLMs.

RecallProxy is a high-performance middleware currently in the conceptual and architectural design phase. It aims to sit between AI Agents and LLM providers to provide a unified abstraction layer for cognitive memory.

The goal is to allow developers to hot-swap and orchestrate different memory engines—such as Semantic, Temporal, or Structural memory—without changing a single line of agent logic.

🛠 The Vision
In the current landscape, memory solutions (Mem0, Cognee, Zep, pgvector) are fragmented and require deep integration with their specific SDKs. RecallProxy is being built to act as a "Neutral Zone" or a "LiteLLM for Memory."

🔌 Plug-and-Play Architecture
We want RecallProxy to be engine-agnostic. By using a standardized API, you can define your memory stack in a configuration file. Want to use Mem0 for user preferences but Cognee for complex knowledge graphs? RecallProxy will handle the routing and synthesis, presenting a single "Context-Enriched" prompt to your LLM.

⚡ High-Performance Gateway (Rust)
Choosing Rust ensures that the gateway introduces near-zero latency. By avoiding Garbage Collection (GC) pauses, RecallProxy provides the predictable performance required for real-time agentic workflows.

🔄 Async Extraction & Hindsight
One of the core features will be the Hindsight Pattern. RecallProxy will manage memory operations asynchronously:

Intercepting Responses: As the LLM streams a response, RecallProxy captures it.

Background Processing: It extracts key facts or state changes and updates the underlying memory engines (like Hindsight or pgvector) without making the user wait for the write operation to finish.

Context Assembly: Before a request even reaches the LLM, the gateway pulls from multiple sources simultaneously to inject the most relevant history and data.

🏗 Planned Functionality
Universal Memory API: A single set of endpoints to read/write memory regardless of the backend engine.

Memory Routing: Smart logic to decide which memory engine (Semantic vs. Structural) holds the answer to a specific agent query.

Streaming Support: Full support for LLM response streaming with transparent "on-the-fly" memory extraction.

Provider Neutrality: Compatible with any OpenAI-compliant endpoint, including LiteLLM, Anthropic, and local LLMs.

🚦 Project Status: Conceptual
RecallProxy is currently being architected. We are defining the core traits and interface standards for the first set of memory engine integrations.

What we are working on:

Defining the MemoryProvider trait in Rust.

Designing the YAML-based orchestration schema.

Prototyping the async extraction pipeline.

📜 License
Distributed under the Apache License 2.0.

Built with ❤️ for the Agentic future.
