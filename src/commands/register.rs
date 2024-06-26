use crate::{Context, Error};

#[poise::command(
    prefix_command,
    track_edits,
    slash_command,
    hide_in_help,
    category = "Admin",
    owners_only
)]
pub async fn register(ctx: Context<'_>) -> Result<(), Error> {
    poise::builtins::register_application_commands_buttons(ctx).await?;
    Ok(())
}
