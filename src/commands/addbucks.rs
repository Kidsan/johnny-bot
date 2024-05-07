use crate::database::BalanceDatabase;
use crate::{Context, Error};
use poise::serenity_prelude::User;
use poise::CreateReply;

///
/// add bucks to a player
///
/// Enter `/add_bucks <player> <amount>`
/// ```
/// /add_bucks @John 50
/// ```
#[poise::command(
    slash_command,
    category = "Admin",
    hide_in_help,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn add_bucks(
    ctx: Context<'_>,
    #[description = "Who to give bucks to"] user: User,
    #[min = 1]
    #[description = "How much to add"]
    amount: i32,
) -> Result<(), Error> {
    if user.bot {
        let reply = {
            CreateReply::default()
                .content("You can't add money to bots..")
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("You can't afford to do that".into());
    }
    let user_id = user.id.to_string();
    let user_balance = ctx.data().db.get_balance(user_id.clone()).await?;
    ctx.data()
        .db
        .set_balance(user_id.clone(), user_balance + amount)
        .await?;
    let reply =
        { CreateReply::default().content(format!("{} was given {} J-Buck(s)", user, amount,)) };
    ctx.send(reply).await?;
    Ok(())
}
