use clap::{Command, Arg, value_parser};

use twilight_http::Client;
use twilight_gateway::{Shard, ShardId, Intents, Event};
use twilight_model::channel::message::MessageFlags;
use twilight_model::channel::message::component::{Component, ActionRow, Button, ButtonStyle};
use twilight_model::application::interaction::{InteractionData};
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType, InteractionResponseData};

use vesper::macros::{command, check};
use vesper::framework::{Framework, DefaultCommandResult, DefaultError};
use vesper::context::SlashContext;

use std::sync::Arc;
use std::sync::RwLock;
use std::path::PathBuf;

mod config;
mod state;
mod secrets;
mod welcome;
mod ebas;

use config::Permission;

pub struct Context {
	config: config::Config,
	secrets: secrets::Secrets,
	state: RwLock<state::State>,
	state_path: PathBuf,
}

#[check]
async fn member_purge_permission(ctx: &SlashContext<Arc<Context>>) -> Result<bool, DefaultError> {
	let permissions = ctx.data.config.member().permission().purge();

	let user = match (&ctx.interaction.user, &ctx.interaction.member) {
		(Some(user), _) => user.id,
		(_, Some(member)) => match &member.user {
			Some(user) => user.id,
			None => panic!("User data in member should be set!"),
		},
		(None, None) => panic!("Either user or member should be set!"),
	};

	let roles = if let Some(member) = &ctx.interaction.member {
		member.roles.clone()
	} else {
		let member = ctx.http_client()
			.guild_member(ctx.data.config.guild(), user).await
			.expect("Couldn't fetch member.")
			.model().await
			.expect("Couldn't serialize member.");

		member.roles
	};

	let permitted = permissions.iter().any(|p| match p {
		Permission::User(u) => &user == u,
		Permission::Role(r) => roles.contains(r),
	});

	if !permitted {
		let response = InteractionResponse {
			kind: InteractionResponseType::ChannelMessageWithSource,
			data: Some(InteractionResponseData {
				content: Some(String::from("You do not have permission to run this command.")),
				flags: Some(MessageFlags::EPHEMERAL),
				..Default::default()
			}),
		};

		let r = ctx.interaction_client.create_response(ctx.interaction.id, &ctx.interaction.token, &response).await;
		if r.is_err() {
			eprintln!("Something went wrong when responding to command.");
		}
	}

	Ok(permitted)
}

#[command(chat, name = "verify")]
#[description = "Verify your membership"]
async fn member_verify(
	ctx: &mut SlashContext<Arc<Context>>,
	#[description = "The email you used when registering"]
	email: String
) -> DefaultCommandResult {
	let is_member = ebas::verify_membership(Arc::clone(ctx.data), email).await;

	let response = if is_member {
		let user = match (&ctx.interaction.user, &ctx.interaction.member) {
			(Some(user), _) => user,
			(_, Some(member)) => match &member.user {
				Some(user) => user,
				None => panic!("User data in member should be set!"),
			},
			(None, None) => panic!("Either user or member should be set!"),
		};

		let guild = ctx.data.config.guild();
		let user = user.id;
		let role = ctx.data.config.member().role();

		// NOTE This requires the MANAGE_ROLES permission when adding the bot to a guild.
		ctx.http_client().add_guild_member_role(guild, user, role).await.expect("Couldn't add role to member.");

		InteractionResponse {
			kind: InteractionResponseType::ChannelMessageWithSource,
			data: Some(InteractionResponseData {
				content: Some(format!("Thanks for your membership! You have been added to <@&{}>.", role)),
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

	let r = ctx.interaction_client.create_response(ctx.interaction.id, &ctx.interaction.token, &response).await;
	if r.is_err() {
		eprintln!("Something went wrong when responding to command.");
	}

	Ok(())
}

#[command(chat, name = "purge")]
#[description = "Remove all users from the membership role"]
#[checks(member_purge_permission)]
async fn member_purge(ctx: &mut SlashContext<Arc<Context>>) -> DefaultCommandResult {
	let id = ctx.interaction.id;

	let guild = ctx.data.config.guild();
	let role = ctx.data.config.member().role();

	let response = InteractionResponse {
		kind: InteractionResponseType::ChannelMessageWithSource,
		data: Some(InteractionResponseData {
			content: Some(format!("Are you sure that you want to remove *all* members from <@&{}>?", role)),
			components: Some(
				vec![Component::ActionRow(ActionRow {
					components: vec![
						Component::Button(Button {
							custom_id: Some(format!("{}:member_purge_cancel", id)),
							label: Some(String::from("Cancel")),
							style: ButtonStyle::Secondary,
							disabled: false,
							emoji: None,
							url: None,
						}),
						Component::Button(Button {
							custom_id: Some(format!("{}:member_purge_confirm", id)),
							label: Some(String::from("Purge")),
							style: ButtonStyle::Danger,
							disabled: false,
							emoji: None,
							url: None,
						})],
				})]),
			flags: Some(MessageFlags::EPHEMERAL),
			..Default::default()
		}),
	};

	let r = ctx.interaction_client.create_response(ctx.interaction.id, &ctx.interaction.token, &response).await;
	if r.is_err() {
		eprintln!("Something went wrong when responding to command.");
	}

	let interaction = ctx.wait_interaction(move |interaction| {
		if let Some(InteractionData::MessageComponent(data)) = &interaction.data {
			if data.custom_id.starts_with(&id.to_string()) {
				return true;
			}
		}
		false
	}).await.expect("Error waiting for member purge response.");

	let action = if let Some(InteractionData::MessageComponent(data)) = &interaction.data {
		// SAFETY We know that the interaction starts with the id, so we can split at colon to get the action.
		let (_, action) = data.custom_id.split_once(':').unwrap();
		action
	} else {
		unreachable!()
	};

	match action {
		"member_purge_confirm" => {
			let response = format!("I am collecting the necessary data to remove all users from <@&{}>.", role);

			let r = ctx.interaction_client.update_response(&ctx.interaction.token)
				.content(Some(&response)).expect("Response content was malformed.")
				.components(None).expect("Components was malformed.")
				.await;
			if r.is_err() {
				eprintln!("Something went wrong when responding to command.");
			}
		},
		"member_purge_cancel" => {
			let r = ctx.interaction_client.update_response(&ctx.interaction.token)
				.content(Some("Better safe than sorry!")).expect("Response content was malformed.")
				.components(None).expect("Components was malformed.")
				.await;
			if r.is_err() {
				eprintln!("Something went wrong when responding to command.");
			}
			return Ok(());
		},
		_ => unreachable!(),
	}

	// Get all members in the guild.
	let mut members = Vec::new();
	let mut after = None;
	const MEMBER_LIMIT: u16 = 1000;

	loop {
		// NOTE This requires the GUILD_MEMBERS priviledged intent.
		let mut req = ctx.http_client()
			.guild_members(guild);
		req = req.limit(MEMBER_LIMIT).expect("Invalid limit.");
		if let Some(id) = after {
			req = req.after(id);
		}

		let mut users =
			req.await.expect("Couldn't get members.")
			.models().await.expect("Couldn't serialize response.");

		// If we didn't get any users
		if users.is_empty() {
			break;
		}

		// SAFETY We can unwrap since we breaked if the list was empty.
		let last_id = users.last().unwrap().user.id;

		after = Some(last_id);

		// If we received less users than we requested,
		// then we won't get any more in a subsequent request.
		let done = users.len() < MEMBER_LIMIT as usize;

		// Retain the users that have the member role.
		users.retain(|user| user.roles.contains(&role));

		members.append(&mut users);

		if done {
			break;
		}
	}

	let content = format!("Found {0} members in <@&{1}>. Do you want me to remove them from <@&{1}>?", members.len(), role);
	let buttons = vec![Component::ActionRow(ActionRow {
					components: vec![
						Component::Button(Button {
							custom_id: Some(format!("{}:member_purge_cancel", id)),
							label: Some(String::from("Cancel")),
							style: ButtonStyle::Secondary,
							disabled: false,
							emoji: None,
							url: None,
						}),
						Component::Button(Button {
							custom_id: Some(format!("{}:member_purge_confirm", id)),
							label: Some(String::from("Continue")),
							style: ButtonStyle::Danger,
							disabled: false,
							emoji: None,
							url: None,
						})],
				})];
	let message = ctx.interaction_client.create_followup(&ctx.interaction.token)
		.content(&content).expect("Response content was malformed.")
		.components(&buttons).expect("Components was malformed.")
		.flags(MessageFlags::EPHEMERAL)
		.await.expect("Something went wrong when responding to command.")
		.model().await.expect("Couldn't deserialize message.");
	let confirmation_message_id = message.id;

	let interaction = ctx.wait_interaction(move |interaction| {
		if let Some(InteractionData::MessageComponent(data)) = &interaction.data {
			if data.custom_id.starts_with(&id.to_string()) {
				return true;
			}
		}
		false
	}).await.expect("Error waiting for member purge response.");

	let action = if let Some(InteractionData::MessageComponent(data)) = &interaction.data {
		// SAFETY We know that the interaction starts with the id, so we can split at colon to get the action.
		let (_, action) = data.custom_id.split_once(':').unwrap();
		action
	} else {
		unreachable!()
	};

	match action {
		"member_purge_confirm" => {
			let r = ctx.interaction_client.update_followup(&ctx.interaction.token, confirmation_message_id)
				.content(Some(&format!("I will remove {} members from <@&{}>.", members.len(), role))).expect("Response content was malformed.")
				.components(None).expect("Components was malformed.")
				.await;
			if r.is_err() {
				eprintln!("Something went wrong when responding to command.");
			}
		},
		"member_purge_cancel" => {
			let r = ctx.interaction_client.update_followup(&ctx.interaction.token, confirmation_message_id)
				.content(Some("Won't proceed with deleting the members.")).expect("Response content was malformed.")
				.components(None).expect("Components was malformed.")
				.await;
			if r.is_err() {
				eprintln!("Something went wrong when responding to command.");
			}
			return Ok(());
		},
		_ => unreachable!(),
	}

	// Remove the role from each member.
	for member in members {
		let user = member.user.id;
		// NOTE This requires the MANAGE_ROLES permission when adding the bot to a guild.
		ctx.http_client()
			.remove_guild_member_role(guild, user, role)
			.await.expect("Couldn't remove role from member.");
	}

	// Say that we are done with the purge.
	let r = ctx.interaction_client.create_followup(&ctx.interaction.token)
		.content(&format!("I have removed all members from <@&{}>.", role)).expect("Response content was malformed.")
		.flags(MessageFlags::EPHEMERAL)
		.await;
	if r.is_err() {
		eprintln!("Something went wrong when responding to command.");
	}

	Ok(())
}

#[tokio::main]
async fn main() {
	let cli = Command::new("kodbot")
		.arg(Arg::new("config")
			.long("config")
			.required(false)
			.value_name("FILE")
			.default_value(config::DEFAULT_PATH)
			.value_parser(value_parser!(PathBuf)))
		.arg(Arg::new("secrets")
			.long("secrets")
			.required(false)
			.value_name("FILE")
			.default_value(secrets::DEFAULT_PATH)
			.value_parser(value_parser!(PathBuf)))
		.arg(Arg::new("state")
			.long("state")
			.required(false)
			.value_name("FILE")
			.default_value(state::DEFAULT_PATH)
			.value_parser(value_parser!(PathBuf)));

	let matches = cli.get_matches();

	// SAFETY We supplied a default value to the argument, so this is always Some.
	let secrets_path = matches.get_one::<PathBuf>("secrets").unwrap();
	let config_path = matches.get_one::<PathBuf>("config").unwrap();
	let state_path = matches.get_one::<PathBuf>("state").unwrap();

	let secrets = std::fs::read_to_string(secrets_path).expect("Couldn't read secrets.");
	let secrets: secrets::Secrets = toml::from_str(&secrets).expect("Couldn't read secrets.");

	let config = std::fs::read_to_string(config_path).expect("Couldn't read configuration.");
	let config: config::Config = toml::from_str(&config).expect("Couldn't read configuration.");

	let state = match state::from_file(state_path) {
		Ok(state) => state,
		Err(e) => match e {
			state::StateError::NotFound => state::State::new(),
			_ => panic!("Failed to read state!"),
		},
	};

	let context = Arc::new(Context {
		config: config,
		secrets: secrets,
		state: RwLock::new(state),
		state_path: state_path.clone(),
	});

	let client = Arc::new(Client::new(context.secrets.discord.token.clone()));

	welcome::handle_welcome_message(&client, Arc::clone(&context)).await;

	let framework = Arc::new(Framework::builder(Arc::clone(&client), context.secrets.discord.application, Arc::clone(&context))
		.group(|g| g
			.name("member")
			.description("INSERT DESC")
			.command(member_verify)
			.command(member_purge))
		.build());

	let result = framework.register_guild_commands(context.config.guild()).await;
	if result.is_err() {
		panic!("Failed to register commands!");
	}

	let mut shard = Shard::new(ShardId::ONE, context.secrets.discord.token.clone(), Intents::empty());

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

		tokio::spawn(event_handler(event, Arc::clone(&framework)));
	}
}

async fn event_handler(event: Event, framework: Arc<Framework<Arc<Context>>>) {
	match event {
		Event::InteractionCreate(interaction) => {
			let interaction = interaction.0;
			framework.process(interaction).await;
		},
		_ => (),
	}
}