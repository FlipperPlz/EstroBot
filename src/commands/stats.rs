use serenity::all::{CommandInteraction, CreateInteractionResponse, CreateInteractionResponseMessage};
use anyhow::Result;
use crate::data::store::Store;

pub fn register() -> serenity::builder::CreateCommand {
    serenity::builder::CreateCommand::new("stats")
        .description("View your hormone stats")
}

pub async fn handle_command(ctx: &serenity::prelude::Context, command: &CommandInteraction, store: &Store) -> Result<()> {
    let user_id = command.user.id.get();
    let content = match store.load_profile(user_id) {
        Ok(Some(profile)) => profile.get_stats(),
        Ok(None) => "You don't have a profile setup yet! Use `/setup` to create one.".to_string(),
        Err(e) => format!("Error loading profile: {}", e),
    };

    command.create_response(&ctx.http, CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content(content)
            .ephemeral(true)
    )).await?;

    Ok(())
}
