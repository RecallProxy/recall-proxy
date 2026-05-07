//! Request-path orchestration definitions for context assembly.

use std::future::Future;
use std::pin::Pin;

use recall_proxy_core::context::{
    AssembledContext, ContextRequest, EngineLookupResult, TokenBudget,
};

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait ContextEngineProvider: Send + Sync {
    fn name(&self) -> &str;

    fn lookup_context<'a>(&'a self, request: &'a ContextRequest) -> BoxFuture<'a, EngineLookupResult>;
}

pub trait ContextAssembler: Send + Sync {
    fn assemble(
        &self,
        request: &ContextRequest,
        token_budget: &TokenBudget,
        results: Vec<EngineLookupResult>,
    ) -> AssembledContext;
}

/// Orchestration flow:
/// 1) Validate request and derive token budget.
/// 2) Fan-out in parallel to configured providers.
/// 3) Join with timeout and collect partial failures.
/// 4) Merge snippets with deterministic precedence.
/// 5) Return prompt-ready context payload.
pub struct RequestContextOrchestrator<A> {
    pub assembler: A,
}

impl<A> RequestContextOrchestrator<A>
where
    A: ContextAssembler,
{
    pub fn execute(
        &self,
        request: &ContextRequest,
        token_budget: &TokenBudget,
        results: Vec<EngineLookupResult>,
    ) -> AssembledContext {
        self.assembler.assemble(request, token_budget, results)
    }
}
