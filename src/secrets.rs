use twilight_model::id::Id;
use twilight_model::id::marker::ApplicationMarker;

use serde::{Serialize, Deserialize};

pub const DEFAULT_PATH: &str = "secrets.toml";

#[derive(Serialize, Deserialize, Clone)]
pub struct Secrets {
	pub discord: Discord,
	pub ebas: Ebas,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Discord {
	pub token: String,
	pub application: Id<ApplicationMarker>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Ebas {
	pub api_key: String,
	pub id: String,
}