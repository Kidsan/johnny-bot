use crate::{Context, Error};
use poise::CreateReply;
///
/// restart the bot
///
/// Enter `/restart`
/// ```
/// /restart
/// ```
#[poise::command(
    slash_command,
    rename = "restart",
    category = "Admin",
    hide_in_help,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn quit(ctx: Context<'_>) -> Result<(), Error> {
    let reply = { CreateReply::default().content("success").ephemeral(true) };
    ctx.send(reply).await?;
    std::process::exit(1);
}
