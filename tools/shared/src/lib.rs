//! # TPT AI — Multi-provider AI abstraction
//!
//! Unified `AiProvider` trait for LLM inference across multiple backends:
//! - **Claude** (Anthropic API)
//! - **OpenRouter** (aggregated multi-model API)
//! - **Ollama** (local model serving)
//!
//! Each provider implements the same trait, allowing kernel-generation prompts,
//! optimization hints, and natural-language queries to route through whichever
//! backend is configured.
//!
//! ## Backward-compatible `generate()` convenience method
//!
//! The [`AiProvider::generate`] method provides a simple `generate(&str) -> Result<String, AiError>`
//! convenience that wraps the structured [`AiProvider::complete`] API.  This is used by
//! `tools/kernel-optimizer` and `tools/kernel-generator` for simple prompt-to-response
//! flows without needing to construct full [`AiRequest`] objects.

pub mod error;
pub mod providers;
pub mod request;
pub mod response;

pub use error::{AiError, AiResult};
pub use request::{AiRequest, AiMessage, Role, ModelConfig};
pub use response::{AiResponse, AiChoice, Usage, FinishReason};
pub use providers::{
    AiProvider, claude::ClaudeProvider, openrouter::OpenRouterProvider,
    ollama::OllamaProvider, ProviderFactory, available_providers, is_valid_provider,
};

/// Re-export commonly used types
pub mod prelude {
    pub use crate::{AiProvider, AiRequest, AiResponse, AiError, AiResult};
    pub use crate::request::{AiMessage, Role, ModelConfig};
}

/// Crate version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// ---------------------------------------------------------------------------
// Provider discovery
// ---------------------------------------------------------------------------

/// Select a provider from environment variables.
///
/// Priority: ANTHROPIC_API_KEY → OPENROUTER_API_KEY → Ollama (local, no key needed).
///
/// Returns a boxed trait object that supports both the structured [`AiProvider::complete`]
/// API and the simple [`AiProvider::generate`] convenience method.
pub fn provider_from_env() -> Box<dyn AiProvider> {
    if let Ok(provider) = ClaudeProvider::from_env() {
        Box::new(provider)
    } else if let Ok(provider) = OpenRouterProvider::from_env() {
        Box::new(provider)
    } else {
        Box::new(OllamaProvider::new())
    }
}
