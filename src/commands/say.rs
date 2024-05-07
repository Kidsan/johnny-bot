use crate::{Context, Error};
use poise::CreateReply;
///
/// Have Johnny say something
///
/// Enter `/say <message>` to make Johnny say something
/// ```
/// /say Awoo
/// ```
///
#[poise::command(
    slash_command,
    category = "Admin",
    default_member_permissions = "ADMINISTRATOR",
    hide_in_help
)]
pub async fn say(
    ctx: Context<'_>,
    #[description = "What to say?"] message: String,
) -> Result<(), Error> {
    let reply = { CreateReply::default().content("Success!").ephemeral(true) };
    ctx.send(reply).await?;
    ctx.channel_id().say(ctx, message).await?;
    Ok(())
}
