use std::sync::Arc;

use rig::{client::ProviderClient, providers::gemini};

mod json_to_typst;
pub mod orgenizer;


pub fn gemini_client() -> Arc<gemini::client::Client> {
    Arc::new(gemini::Client::from_env())
}