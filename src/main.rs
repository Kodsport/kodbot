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
	ctx: &SlashContext<Arc<Context>>,
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
		let role = ctx.data.state.read().unwrap().member().unwrap().role();

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
async fn member_purge(ctx: &SlashContext<Arc<Context>>) -> DefaultCommandResult {
	let response = InteractionResponse {
		kind: InteractionResponseType::ChannelMessageWithSource,
		data: Some(InteractionResponseData {
			content: Some(format!("Are you sure that you want to remove *all* members from <@&{}>?", ctx.data.state.read().unwrap().member().unwrap().role())),
			components: Some(
				vec![Component::ActionRow(ActionRow {
					components: vec![
						Component::Button(Button {
							custom_id: Some(String::from("member_purge_cancel")),
							label: Some(String::from("Cancel")),
							style: ButtonStyle::Secondary,
							disabled: false,
							emoji: None,
							url: None,
						}),
						Component::Button(Button {
							custom_id: Some(String::from("member_purge_confirm")),
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

	let interaction = ctx.wait_interaction(|interaction| {
		if let Some(InteractionData::MessageComponent(data)) = &interaction.data {
			data.custom_id == "member_purge_confirm" || data.custom_id == "member_purge_cancel"
		} else {
			false
		}
	}).await;

	let interaction = interaction.expect("Error waiting for member purge response.");

	let response = if let Some(InteractionData::MessageComponent(data)) = &interaction.data {
		match data.custom_id.as_str() {
			"member_purge_confirm" => {
				// There are two ways to purge:
				// 1. Get all guild members,
				//    filter out those that belong to the member role,
				//    remove them from the role.
				// 2. Remove the role entirely and recreate it.
				// The second option should be easier to implement and less prone to errors.
				// Plus, Discord doesn't have to send all member data to us.
				// Before deleting the role, we will fetch the current information,
				// so we can feed that into the creation in order to preserve any manual changes in the guild.

				let guild = ctx.data.config.guild();
				let role = ctx.data.state.read().unwrap().member().unwrap().role();

				// NOTE This requires the MANAGE_ROLES permission when adding the bot to a guild.
				let roles = ctx.http_client().roles(guild).await
					.expect("Couldn't fetch roles.")
					.models().await
					.expect("Couldn't serialize response.");

				// SAFETY At startup we made sure that the member role existed,
				// so we can still expect the role to exist.
				let role_info = roles.iter().find(|r| r.id == role).unwrap();

				ctx.http_client().delete_role(guild, role).await.expect("Couldn't delete role.");

				// TODO Some properties aren't forwarded here.
				let role = ctx.http_client().create_role(guild)
					.color(role_info.color)
					.hoist(role_info.hoist)
					.mentionable(role_info.mentionable)
					.name(role_info.name.as_str())
					.permissions(role_info.permissions).await
					.expect("Couldn't create role")
					.model().await
					.expect("Couldn't serialize response.");
				
				let role = role.id;

				let state = &mut ctx.data.state.write().unwrap();

				state.member_mut().unwrap().set_role(role);

				if let Err(_) = state::to_file(crate::state::DEFAULT_PATH, &state) {
					eprintln!("Couldn't write state to file!");
				}

				"The member role was successfully purged. Remember to tell people to register for a new membership!"
			},
			"member_purge_cancel" => "Better safe than sorry!",
			_ => unreachable!(),
		}
	} else {
		unreachable!()
	};

	let r = ctx.interaction_client.update_response(&ctx.interaction.token)
		.content(Some(response)).expect("Response content was malformed.")
		.components(None).expect("Components was malformed.")
		.await;
	if r.is_err() {
		eprintln!("Something went wrong when responding to command.");
	}

	Ok(())
}

#[tokio::main]
async fn main() {
	let s = std::fs::read_to_string(secrets::DEFAULT_PATH).expect("Couldn't read secrets.");
	let secrets: secrets::Secrets = toml::from_str(&s).expect("Couldn't read secrets.");

	let s = std::fs::read_to_string(config::DEFAULT_PATH).expect("Couldn't read configuration.");
	let config: config::Config = toml::from_str(&s).expect("Couldn't read configuration.");

	let state = match state::from_file(state::DEFAULT_PATH) {
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
	});

	let client = Arc::new(Client::new(context.secrets.discord.token.clone()));

	welcome::handle_welcome_message(&client, Arc::clone(&context)).await;

	{
		let name = context.config.member().name();
		// NOTE This requires the MANAGE_ROLES permission when adding the bot to a guild.
		let roles = client.roles(context.config.guild()).await
			.expect("Couldn't fetch roles.")
			.models().await
			.expect("Couldn't serialize response.");

		let role = roles.iter().find(|role| &role.name == name);

		let role = if let Some(role) = role {
			role.id
		} else {
			// The role does not exist in the guild, so we create it.
			// NOTE This requires the MANAGE_ROLES permission when adding the bot to a guild.
			let role = client.create_role(context.config.guild()).name(name).await
				.expect("Couldn't create role")
				.model().await
				.expect("Couldn't serialize response.");
			role.id
		};

		let state = &mut context.state.write().unwrap();

		if let Some(member) = state.member_mut() {
			if member.role() != role {
				member.set_role(role);
			}
		} else {
			state.set_member(state::Member::new(role));
		}

		if let Err(_) = state::to_file(crate::state::DEFAULT_PATH, state) {
			eprintln!("Couldn't write state to file!");
		}
	}

	let framework = Arc::new(Framework::builder(Arc::clone(&client), context.secrets.discord.application, Arc::clone(&context))
		.group(|g| g
			.name("member")
			.description("INSERT DESC")
			.command(member_verify)
			/*.command(member_purge)*/)
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