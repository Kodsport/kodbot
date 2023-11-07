use twilight_http::Client;
use twilight_gateway::{Shard, ShardId, Intents, Event};
use twilight_util::builder::command::{CommandBuilder, SubCommandBuilder, StringBuilder};
use twilight_model::application::command::{Command, CommandType};
use twilight_model::application::interaction::InteractionData;
use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::channel::message::MessageFlags;
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

	guild_commands.push(
		CommandBuilder::new("member", "command for handling members (UPDATE THIS DESCRIPTION!)", CommandType::ChatInput)
			.guild_id(config.guild())
			.option(SubCommandBuilder::new("verify", "Verify your membership")
				.option(StringBuilder::new("email", "The email you used when registering").required(true).build())
				.build())
			.build());

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

		tokio::spawn(event_handler(event, Arc::clone(&client), config.clone(), secrets.clone()));
	}
}

async fn event_handler(event: Event, client: Arc<Client>, config: config::Config, secrets: secrets::Secrets) {
	match event {
		Event::InteractionCreate(interaction) => {
			if let InteractionData::ApplicationCommand(command) = interaction.data.clone().unwrap() { // TODO Don't unwrap
				if command.name == "member" {
					let subcommand = command.options.first().unwrap();
					match subcommand.name.as_str() {
						"verify" => {
							if let CommandOptionValue::SubCommand(data) = &subcommand.value {
								let email = match &data.first().unwrap().value {
									CommandOptionValue::String(email) => email,
									_ => panic!("wrong option type!"),
								};

								let is_member = ebas::verify_membership(email.clone(), &config, &secrets).await;

								let response = if is_member {
									let user = match (&interaction.user, &interaction.member) {
										(Some(user), _) => user,
										(_, Some(member)) => &member.user.as_ref().expect("User data in member should be set!"),
										(None, None) => panic!("Either user or member should be set!"),
									};

									// NOTE This requires the MANAGE_ROLES permission when adding the bot to a guild.
									client.add_guild_member_role(config.guild(), user.id, config.member().role()).await.expect("Couldn't add role to member.");

									InteractionResponse {
										kind: InteractionResponseType::ChannelMessageWithSource,
										data: Some(InteractionResponseData {
											content: Some(format!("Thanks for your membership! You have been added to <@&{}>.", config.member().role())),
											flags: Some(MessageFlags::EPHEMERAL),
											..Default::default()
										}),
									}
								} else {
									InteractionResponse {
										kind: InteractionResponseType::ChannelMessageWithSource,
										data: Some(InteractionResponseData {
											content: Some(String::from("We have no registered member with this email. After you have registered, you can rerun the command.")),
											flags: Some(MessageFlags::EPHEMERAL),
											..Default::default()
										}),
									}
								};

								let interaction_client = client.interaction(secrets.discord.application);

								let r = interaction_client.create_response(interaction.id, &interaction.token, &response).await;
								if r.is_err() {
									eprintln!("Something went wrong when responding to command.");
								}
							}
						},
						name => panic!("No such subcommand {}", name),
					}
				}
			}
		},
		_ => (),
	}
}