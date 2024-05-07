use crate::database::BalanceDatabase;
use crate::{Context, Error};
use poise::serenity_prelude::User;
use poise::CreateReply;
///
/// Remove bucks from a player
///
/// ```
/// /remove_bucks @John 50
/// ```
#[poise::command(
    slash_command,
    category = "Admin",
    hide_in_help,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn remove_bucks(
    ctx: Context<'_>,
    #[description = "Who to remove from"] user: User,
    #[min = 1]
    #[description = "How much to remove"]
    amount: i32,
) -> Result<(), Error> {
    if user.bot {
        let reply = {
            CreateReply::default()
                .content("You can't remove money from bots..")
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("You can't afford to do that".into());
    }
    let user_id = user.id.to_string();
    let user_balance = ctx.data().db.get_balance(user_id.clone()).await?;
    if user_balance < amount {
        let reply = {
            CreateReply::default()
                .content(format!(
                    "They can't afford to do that!\n{}'s balance is only {} J-Buck(s)",
                    user, user_balance
                ))
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("can't afford to do that".into());
    }
    ctx.data()
        .db
        .set_balance(user_id.clone(), user_balance - amount)
        .await?;

    let reply =
        { CreateReply::default().content(format!("Removed {} J-Bucks from {}", amount, user,)) };
    ctx.send(reply).await?;
    Ok(())
}
