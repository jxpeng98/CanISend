use std::time::Duration;

use reqwest::redirect::Policy;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::builder()
        .redirect(Policy::none())
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(10))
        .user_agent("canisend-r0-network-probe")
        .build()?;

    if std::env::var_os("CANISEND_R0_NETWORK_GET").is_some() {
        let response = client.get("https://example.com/").send().await?;
        if !response.status().is_success() {
            return Err(format!("HTTPS probe returned {}", response.status()).into());
        }
    }

    println!("reqwest-rustls-client-ok");
    Ok(())
}
