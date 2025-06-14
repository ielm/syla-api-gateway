pub mod execution;

use tonic::transport::{Channel, Endpoint};
use anyhow::Result;

// Create a shared channel for a service
pub async fn create_channel(url: &str) -> Result<Channel> {
    let endpoint = Endpoint::from_shared(url.to_string())?
        .connect_timeout(std::time::Duration::from_secs(5))
        .timeout(std::time::Duration::from_secs(30));
    
    let channel = endpoint.connect().await?;
    Ok(channel)
}