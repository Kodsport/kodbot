use twilight_http::Client;
use twilight_model::id::{Id, marker::{ChannelMarker, MessageMarker}};
use twilight_model::channel::Message;

pub async fn post_welcome_message<M: AsRef<str>>(client: &Client, channel: Id<ChannelMarker>, message: M) -> Message {
	client
		.create_message(channel)
		.content(message.as_ref()).expect("Message was malformed.")
		.await.expect("Couldn't send welcome message.")
		.model().await.expect("Couldn't deserialize message from response.")
}

pub fn store_welcome_message_id(id: Id<MessageMarker>) {
	let mut state = std::fs::read_to_string("state.toml").unwrap_or(String::new())
		.parse::<toml_edit::Document>().expect("Couldn't read state file.");

	if !state.contains_table("welcome") {
		state["welcome"] = toml_edit::table();
	}

	state["welcome"]["message"] = toml_edit::value(id.get() as i64);

	std::fs::write("state.toml", state.to_string()).expect("Couldn't write state file.");
}