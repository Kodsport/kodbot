use twilight_http::Client;
use twilight_model::id::Id;

mod state;
mod secrets;
mod welcome;

#[tokio::main]
async fn main() {
    let s = std::fs::read_to_string(secrets::DEFAULT_PATH).expect("Couldn't read secrets.");
    let secrets: secrets::Secrets = toml::from_str(&s).expect("Couldn't read secrets.");

    let path = "config.toml";
    let config = std::fs::read_to_string(path).ok()
        .and_then(|c| c.parse::<toml::Table>().ok())
        .expect(&format!("Couldn't read configuration form {path}"));

    let mut state = match state::from_file(state::DEFAULT_PATH) {
        Ok(state) => state,
        Err(e) => match e {
            state::StateError::NotFound => state::State::new(),
            _ => panic!("Failed to read state!"),
        },
    };

    let client = Client::new(String::from(secrets.token));

    if state.welcome().is_none() {
        let channel_id = Id::new(
            config.get("welcome")
            .and_then(|w| w.as_table())
            .and_then(|w| w.get("channel"))
            .and_then(|c| c.as_integer())
            .and_then(|c| c.try_into().ok()) // NOTE Discord IDs are u64 and toml parsing can only handle i64, but IDs are currently < i64 max.
            .expect("Couldn't get channel for welcome message."));

        let message_path = config.get("welcome")
            .and_then(|w| w.as_table())
            .and_then(|w| w.get("file"))
            .and_then(|p| p.as_str())
            .expect("Couldn't get path for welcome message.");
        let message = std::fs::read_to_string(message_path).expect(&format!("Couldn't read message from {message_path}."));
        let message = welcome::post_welcome_message(&client, channel_id, message).await;
        state.set_welcome(state::Welcome::new(message.id));
        if let Err(_) = state::to_file(state::DEFAULT_PATH, &state) {
            eprintln!("Couldn't write state to file!");
        }
    }
}
