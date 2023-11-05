use twilight_http::Client;

#[tokio::main]
async fn main() {
    let path = "secrets.toml";
    let secrets = std::fs::read_to_string(path).ok()
        .and_then(|c| c.parse::<toml::Table>().ok())
        .expect(&format!("Couldn't read secrets from {path}"));

    let path = "config.toml";
    let config = std::fs::read_to_string(path).ok()
        .and_then(|c| c.parse::<toml::Table>().ok())
        .expect(&format!("Couldn't read configuration form {path}"));

    let token = secrets.get("token").and_then(|v| v.as_str()).expect("Couldn't read token from secrets.");
    let client = Client::new(String::from(token));
}
