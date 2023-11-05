use serenity::client::Client;
use serenity::model::gateway::GatewayIntents;

#[tokio::main]
async fn main() {
    let path = "secrets.toml";
    let secrets = std::fs::read_to_string(path).ok()
        .and_then(|c| c.parse::<toml::Table>().ok())
        .expect(&format!("Couldn't read secrets from {path}"));

    let token = secrets.get("token").and_then(|v| v.as_str()).expect("Couldn't read token from secrets.");
    let mut client = Client::builder(token, GatewayIntents::default()).await.expect("Failed to start client.");

    if client.start().await.is_err() {
        eprintln!("Client stopped unexpectedly!");
    }
}
