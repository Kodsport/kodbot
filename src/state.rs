use twilight_model::id::Id;
use twilight_model::id::marker::MessageMarker;

use serde::{Serialize, Deserialize};

pub const DEFAULT_PATH: &str = "state.toml";

// WARNING The state is written when changed and read when the bot starts.
// If this data structure is changed, the stored file have to be updated to
// reflect those changes before the bot starts.
#[derive(Deserialize, Serialize)]
pub struct State {
	welcome: Option<Welcome>,
}

#[derive(Deserialize, Serialize)]
pub struct Welcome {
	message: Id<MessageMarker>,
}

pub enum StateError {
	NotFound,
	Other,
}

use std::convert::From;
impl From<std::io::Error> for StateError {
	fn from(error: std::io::Error) -> StateError {
		match error.kind() {
			std::io::ErrorKind::NotFound => StateError::NotFound,
			_ => StateError::Other
		}
	}
}

impl From<toml::de::Error> for StateError {
	fn from(_: toml::de::Error) -> StateError {
		StateError::Other
	}
}

impl From<toml::ser::Error> for StateError {
	fn from(_: toml::ser::Error) -> StateError {
		StateError::Other
	}
}

impl State {
	pub fn new() -> State {
		State {
			welcome: None,
		}
	}

	pub fn welcome(&self) -> Option<&Welcome> {
		self.welcome.as_ref()
	}

	pub fn welcome_mut(&mut self) -> Option<&mut Welcome> {
		self.welcome.as_mut()
	}

	pub fn set_welcome(&mut self, welcome: Welcome) {
		self.welcome = Some(welcome);
	}
}

impl Welcome {
	pub fn new(message: Id<MessageMarker>) -> Welcome {
		Welcome {
			message,
		}
	}

	pub fn message(&self) -> Id<MessageMarker> {
		self.message
	}

	pub fn set_message(&mut self, message: Id<MessageMarker>) {
		self.message = message
	}
}

pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<State, StateError> {
	let s = std::fs::read_to_string(&path)?;
	Ok(toml::from_str(&s)?)
}

pub fn to_file<P: AsRef<std::path::Path>>(path: P, state: &State) -> Result<(), StateError> {
	let s = toml::to_string(state)?;
	Ok(std::fs::write(&path, s)?)
}