use twilight_model::id::Id;
use twilight_model::id::marker::{GuildMarker, ChannelMarker, UserMarker, RoleMarker};

use serde::{Serialize, Deserialize};

pub const DEFAULT_PATH: &str = "config.toml";

#[derive(Deserialize, Serialize, Clone)]
pub struct Config {
	guild: Id<GuildMarker>,
	welcome: Welcome,
	ebas: Ebas,
	member: Member,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Welcome {
	channel: Id<ChannelMarker>,
	file: Option<String>,
	text: Option<String>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Ebas {
	url: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Member {
	role: Id<RoleMarker>,
	permission: MemberPermission,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct MemberPermission {
	purge: Vec<Permission>,
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
	User(Id<UserMarker>),
	Role(Id<RoleMarker>),
}

impl Config {
	pub fn guild(&self) -> Id<GuildMarker> {
		self.guild
	}

	pub fn welcome(&self) -> &Welcome {
		&self.welcome
	}

	pub fn ebas(&self) -> &Ebas {
		&self.ebas
	}

	pub fn member(&self) -> &Member {
		&self.member
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

impl Ebas {
	pub fn url(&self) -> &String {
		&self.url
	}
}

impl Member {
	pub fn role(&self) -> Id<RoleMarker> {
		self.role
	}

	pub fn permission(&self) -> &MemberPermission {
		&self.permission
	}
}

impl MemberPermission {
	pub fn purge(&self) -> &Vec<Permission> {
		&self.purge
	}
}