use twilight_http::Client;
use twilight_gateway::{Shard, ShardId, Intents, Event};
use twilight_model::channel::message::MessageFlags;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType, InteractionResponseData};

use vesper::macros::command;
use vesper::framework::{Framework, DefaultCommandResult};
use vesper::context::SlashContext;

use std::sync::Arc;
use std::sync::RwLock;

mod config;
mod state;
mod secrets;
mod welcome;
mod ebas;

pub struct Context {
	config: config::Config,
	secrets: secrets::Secrets,
	state: RwLock<state::State>,
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

		// TODO Check if role with name already exists.
		let role = if let Some(role) = role {
			println!("role already exist");
			role.id
		} else {
			println!("Role doesn't exist, creating...");
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
		println!("Writing new state");
		if let Err(_) = state::to_file(crate::state::DEFAULT_PATH, state) {
			eprintln!("Couldn't write state to file!");
		}
	}

	let framework = Arc::new(Framework::builder(Arc::clone(&client), context.secrets.discord.application, Arc::clone(&context))
		.group(|g| g
			.name("member")
			.description("INSERT DESC")
			.command(member_verify))
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