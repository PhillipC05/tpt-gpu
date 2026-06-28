<<<<<<< Updated upstream
mod provider;
pub use provider::{AiProvider, AiError, ClaudeProvider, OpenRouterProvider, OllamaProvider};

/// Select a provider from environment variables.
///
/// Priority: ANTHROPIC_API_KEY → OPENROUTER_API_KEY → Ollama (local, no key needed).
pub fn provider_from_env() -> Box<dyn AiProvider> {
    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
        Box::new(ClaudeProvider::new(key))
    } else if let Ok(key) = std::env::var("OPENROUTER_API_KEY") {
        Box::new(OpenRouterProvider::new(key))
    } else {
        Box::new(OllamaProvider::default())
    }
}
=======
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

pub mod error;
pub mod providers;
pub mod request;
pub mod response;

pub use error::{AiError, AiResult};
pub use request::{AiRequest, AiMessage, Role, ModelConfig};
pub use response::{AiResponse, AiChoice, Usage, FinishReason};
pub use providers::{AiProvider, claude::ClaudeProvider, openrouter::OpenRouterProvider, ollama::OllamaProvider, ProviderFactory, available_providers, is_valid_provider};

/// Re-export commonly used types
pub mod prelude {
    pub use crate::{AiProvider, AiRequest, AiResponse, AiError, AiResult};
    pub use crate::request::{AiMessage, Role, ModelConfig};
}

/// Crate version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
>>>>>>> Stashed changes
