fn main() {
    let secrets = {
        let path = "secrets.toml";
        let contents = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => {
                eprintln!("Couldn't read secrets from {path}.");
                return;
            },
        };

        match contents.parse::<toml::Table>() {
            Ok(s) => s,
            Err(_) => {
                eprintln!("Couldn't read secrets from {path}.");
                return;
            },
        }
    };
}
