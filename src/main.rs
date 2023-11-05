use twilight_http::Client;

mod config;
mod state;
mod secrets;
mod welcome;

#[tokio::main]
async fn main() {
    let s = std::fs::read_to_string(secrets::DEFAULT_PATH).expect("Couldn't read secrets.");
    let secrets: secrets::Secrets = toml::from_str(&s).expect("Couldn't read secrets.");

    let s = std::fs::read_to_string(config::DEFAULT_PATH).expect("Couldn't read configuration.");
    let config: config::Config = toml::from_str(&s).expect("Couldn't read configuration.");

    let mut state = match state::from_file(state::DEFAULT_PATH) {
        Ok(state) => state,
        Err(e) => match e {
            state::StateError::NotFound => state::State::new(),
            _ => panic!("Failed to read state!"),
        },
    };

    let client = Client::new(String::from(secrets.token));

    welcome::handle_welcome_message(&client, &config, &mut state).await;
}
