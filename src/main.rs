mod models;
mod data;
mod commands;
mod threads;

use std::env;
use std::sync::Arc;

use anyhow::Result;
use serenity::async_trait;
use serenity::model::application::{Command, Interaction};
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use tracing::{error, info};

use crate::data::store::Store;

struct Handler {
    store: Arc<Store>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);

        let commands = vec![
            commands::setup::register(),
            commands::stats::register(),
        ];

        if let Err(e) = Command::set_global_commands(&ctx.http, commands).await {
            error!("Error setting global commands: {}", e);
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Command(command) => {
                match command.data.name.as_str() {
                    "setup" => {
                        if let Err(e) = commands::setup::handle_command(&ctx, &command).await {
                            error!("Error handling setup command: {}", e);
                        }
                    }
                    "stats" => {
                        if let Err(e) = commands::stats::handle_command(&ctx, &command, &self.store).await {
                            error!("Error handling stats command: {}", e);
                        }
                    }

                    _ => {
                        let _ = command.create_response(&ctx.http, serenity::builder::CreateInteractionResponse::Message(
                            serenity::builder::CreateInteractionResponseMessage::new().content("Unknown command").ephemeral(true)
                        )).await;
                    }
                };
            }
            Interaction::Component(component) => {
                if let Err(e) = commands::setup::handle_component(&ctx, &component, &self.store).await {
                    error!("Error handling component: {}", e);
                }
            }
            Interaction::Modal(modal) => {
                if let Err(e) = commands::setup::handle_modal(&ctx, &modal, &self.store).await {
                    error!("Error processing modal: {}", e);
                }
            }
            _ => {}
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    dotenv::dotenv().ok();

    let token = env::var("DISCORD_TOKEN").expect("Expected DISCORD_TOKEN in environment");
    let store = Arc::new(Store::new("data/profiles")?);

    let intents = GatewayIntents::GUILDS 
        | GatewayIntents::GUILD_MESSAGES 
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler { store: store.clone() })
        .await
        .expect("Err creating client");

    let http = client.http.clone();
    tokio::spawn(async move {
        threads::reminder::start_reminder_thread(store, http).await;
    });

    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }

    Ok(())
}
