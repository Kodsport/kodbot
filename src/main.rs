fn main() {
    let path = "secrets.toml";
    let secrets = std::fs::read_to_string(path).ok()
        .and_then(|c| c.parse::<toml::Table>().ok())
        .expect(&format!("Couldn't read secrets from {path}"));
}
