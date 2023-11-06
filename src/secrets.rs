use twilight_model::id::Id;
use twilight_model::id::marker::ApplicationMarker;

use serde::{Serialize, Deserialize};

pub const DEFAULT_PATH: &str = "secrets.toml";

#[derive(Serialize, Deserialize, Clone)]
pub struct Secrets {
	pub token: String,
	pub application_id: Id<ApplicationMarker>,
}