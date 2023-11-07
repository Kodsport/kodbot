use twilight_http::Client;
use twilight_gateway::{Shard, ShardId, Intents, Event};
use twilight_util::builder::command::CommandBuilder;
use twilight_model::application::command::{Command, CommandType};
use twilight_model::application::interaction::InteractionData;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType, InteractionResponseData};

use std::sync::Arc;

mod config;
mod state;
mod secrets;
mod welcome;
mod ebas;

#[tokio::main]
async fn main() {
	let s = std::fs::read_to_string(secrets::DEFAULT_PATH).expect("Couldn't read secrets.");
	let secrets: secrets::Secrets = toml::from_str(&s).expect("Couldn't read secrets.");

	let s = std::fs::read_to_string(config::DEFAULT_PATH).expect("Couldn't read configuration.");
	let config: config::Config = toml::from_str(&s).expect("Couldn't read configuration.");

	let mut state = match state::from_file(state::DEFAULT_PATH) {
		Ok(state) => state,
		Err(e) => match e {
			state::StateError::NotFound => state::State::new(),
			_ => panic!("Failed to read state!"),
		},
	};

	let client = Arc::new(Client::new(secrets.discord.token.clone()));

	welcome::handle_welcome_message(&client, &config, &mut state).await;

	let mut global_commands: Vec<Command> = Vec::new();
	let mut guild_commands: Vec<Command> = Vec::new();

	guild_commands.push(CommandBuilder::new("ping", "send ping receive ...", CommandType::ChatInput).guild_id(config.guild()).build());

	let interaction_client = client.interaction(secrets.discord.application);

	// "Setting" global/guild commands will replace existing commands with the new ones.
	interaction_client.set_global_commands(&global_commands).await.expect("Couldn't set global commands.");
	interaction_client.set_guild_commands(config.guild(), &guild_commands).await.expect("Couldn't set guild commands.");

	let mut shard = Shard::new(ShardId::ONE, secrets.discord.token.clone(), Intents::empty());

	loop {
		let event = match shard.next_event().await {
			Ok(event) => event,
			Err(e) => {
				eprintln!("Encountered error when receiving event.");
				if e.is_fatal() {
					break;
				}

				continue;
			},
		};

		tokio::spawn(event_handler(event, Arc::clone(&client), secrets.clone()));
	}
}

async fn event_handler(event: Event, client: Arc<Client>, secrets: secrets::Secrets) {
	match event {
		Event::InteractionCreate(interaction) => {
			if let InteractionData::ApplicationCommand(command) = interaction.data.clone().unwrap() { // TODO Don't unwrap
				if command.name == "ping" {
					let interaction_client = client.interaction(secrets.discord.application);
					let response = InteractionResponse {
						kind: InteractionResponseType::ChannelMessageWithSource,
						data: Some(InteractionResponseData {
							content: Some(String::from("pong")),
							..Default::default()
						}),
					};

					let r = interaction_client.create_response(interaction.id, &interaction.token, &response).await;
					if r.is_err() {
						eprintln!("Something went wrong when responding to command.");
					}
				}
			}
		},
		_ => (),
	}
}