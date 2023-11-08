use twilight_model::id::Id;
use twilight_model::id::marker::{MessageMarker, RoleMarker};

use serde::{Serialize, Deserialize};

pub const DEFAULT_PATH: &str = "state.toml";

#[derive(Deserialize, Serialize, Clone, Copy)]
pub struct State {
	welcome: Option<Welcome>,
	member: Option<Member>,
}

#[derive(Deserialize, Serialize, Clone, Copy)]
pub struct Welcome {
	message: Id<MessageMarker>,
}

#[derive(Deserialize, Serialize, Clone, Copy)]
pub struct Member {
	role: Id<RoleMarker>,
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
			member: None,
		}
	}

	pub fn welcome(&self) -> &Option<Welcome> {
		&self.welcome
	}

	pub fn welcome_mut(&mut self) -> &mut Option<Welcome> {
		&mut self.welcome
	}

	pub fn set_welcome(&mut self, welcome: Welcome) {
		self.welcome = Some(welcome);
	}

	pub fn member(&self) -> &Option<Member> {
		&self.member
	}

	pub fn member_mut(&mut self) -> &mut Option<Member> {
		&mut self.member
	}

	pub fn set_member(&mut self, member: Member) {
		self.member = Some(member)
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

impl Member {
	pub fn new(role: Id<RoleMarker>) -> Member {
		Member {
			role
		}
	}

	pub fn role(&self) -> Id<RoleMarker> {
		self.role
	}

	pub fn set_role(&mut self, role: Id<RoleMarker>) {
		self.role = role
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