use twilight_http::Client;
use twilight_model::id::{Id, marker::ChannelMarker};
use twilight_model::channel::Message;

pub async fn post_welcome_message<M: AsRef<str>>(client: &Client, channel: Id<ChannelMarker>, message: M) -> Message {
	client
		.create_message(channel)
		.content(message.as_ref()).expect("Message was malformed.")
		.await.expect("Couldn't send welcome message.")
		.model().await.expect("Couldn't deserialize message from response.")
}