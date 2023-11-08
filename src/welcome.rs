use twilight_http::Client;
use twilight_model::id::{Id, marker::{ChannelMarker, MessageMarker}};
use twilight_model::channel::Message;

use std::sync::Arc;
use crate::Context;

pub enum WelcomeError {
	MessageNotFound,
	WrongContent,
	Other,
}

// NOTE This requires the SEND_MESSAGES permission.
pub async fn post_welcome_message<M: AsRef<str>>(client: &Client, channel: Id<ChannelMarker>, content: M) -> Message {
	client
		.create_message(channel)
		.content(content.as_ref()).expect("Message was malformed.")
		.await.expect("Couldn't send welcome message.")
		.model().await.expect("Couldn't deserialize message from response.")
}

// NOTE This doesn't require any permissions, since we only edit our own messages.
pub async fn edit_welcome_message<M: AsRef<str>>(client: &Client, channel: Id<ChannelMarker>, message: Id<MessageMarker>, content: M) {
	client
		.update_message(channel, message)
		.content(Some(content.as_ref())).expect("Message was malformed.")
		.await.expect("Couldn't update welcome message.")
		.model().await.expect("Couldn't deserialize message from response.");
}

pub async fn validate_welcome_message<M: AsRef<str>>(
	client: &Client,
	channel: Id<ChannelMarker>,
	message: Id<MessageMarker>,
	content: M
) -> Result<(), WelcomeError> {
	let response = match client.message(channel, message).await {
		Ok(response) => response,
		Err(e) => return Err(match e.kind() {
			twilight_http::error::ErrorType::Response { .. } => WelcomeError::MessageNotFound,
			_ => WelcomeError::Other,
		})
	};

	let message = response.model().await
		.expect("Couldn't deserialize message from response.");

	if message.content != content.as_ref() {
		return Err(WelcomeError::WrongContent);
	}

	Ok(())
}

pub async fn handle_welcome_message(client: &Client, context: Arc<Context>) {
    let channel = context.config.welcome().channel();
	let content = match context.config.welcome().content() {
		Some(content) => content,
		None => return, // There is no message to handle.
	};

	let state = &mut context.state.write().unwrap();

	if let Some(welcome) = state.welcome_mut() {
		match validate_welcome_message(&client, channel, welcome.message(), &content).await {
			Ok(_) => return,
			Err(e) => match e {
				WelcomeError::MessageNotFound => {
					let message = post_welcome_message(&client, channel, content).await;
					welcome.set_message(message.id);
					if let Err(_) = crate::state::to_file(crate::state::DEFAULT_PATH, state) {
						eprintln!("Couldn't write state to file!");
					}
				},
				WelcomeError::WrongContent => {
					edit_welcome_message(&client, channel, welcome.message(), content).await;
				},
				WelcomeError::Other => return,
			}
		}
	} else {
		let message = post_welcome_message(&client, channel, &content).await;
		state.set_welcome(crate::state::Welcome::new(message.id));
		if let Err(_) = crate::state::to_file(crate::state::DEFAULT_PATH, state) {
			eprintln!("Couldn't write state to file!");
		}
	}
}