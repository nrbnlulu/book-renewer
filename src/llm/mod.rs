use rig::client::ProviderClient;
use rig::providers::gemini;
use std::sync::Arc;

pub fn gemini_client() -> Arc<gemini::client::Client> {
    Arc::new(gemini::Client::from_env())
}
