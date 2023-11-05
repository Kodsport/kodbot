use twilight_model::id::Id;
use twilight_model::id::marker::{GuildMarker, ChannelMarker};

use serde::{Serialize, Deserialize};

pub const DEFAULT_PATH: &str = "config.toml";

#[derive(Deserialize, Serialize)]
pub struct Config {
	guild: Id<GuildMarker>,
	welcome: Welcome,
}

#[derive(Deserialize, Serialize)]
pub struct Welcome {
	channel: Id<ChannelMarker>,
	file: Option<String>,
	text: Option<String>,
}

impl Config {
	pub fn guild(&self) -> Id<GuildMarker> {
		self.guild
	}

	pub fn welcome(&self) -> &Welcome {
		&self.welcome
	}
}

impl Welcome {
	pub fn channel(&self) -> Id<ChannelMarker> {
		self.channel
	}

	pub fn content(&self) -> Option<String> {
		// Start by checking the text key, i.e. it will override the file.
		if let Some(t) = &self.text {
			return Some(t.clone());
		}

		if let Some(f) = &self.file {
			// TODO Throwing away info here by turning it into an Option.
			let content = std::fs::read_to_string(f).ok();
			return content;
		}

		// Neither a file path nor a text was given in the configuration,
		// so we have no welcome message.
		None
	}
}