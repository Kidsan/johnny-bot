use crate::database::BalanceDatabase;
use crate::{Context, Error};
use poise::serenity_prelude as serenity;
use poise::CreateReply;

///
/// Check someone's balance
///
/// Enter `/checkbucks @Name` to check
/// ```
/// /checkbucks @John
/// ```
///
#[poise::command(
    slash_command,
    category = "Admin",
    default_member_permissions = "ADMINISTRATOR",
    hide_in_help
)]
pub async fn checkbucks(
    ctx: Context<'_>,
    #[description = "Who to check"] user: serenity::User,
) -> Result<(), Error> {
    let response = match user.bot {
        true => 0,
        false => {
            ctx.data()
                .db
                .get_balance(user.id.get().try_into().unwrap())
                .await?
        }
    };
    let reply = {
        CreateReply::default()
            .content(format!("{} has {} J-Buck(s)!", user, response,))
            .ephemeral(true)
    };
    ctx.send(reply).await?;
    Ok(())
}
