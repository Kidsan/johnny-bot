use crate::database::BalanceDatabase;
use crate::{Context, Error};
use poise::serenity_prelude::User;
use poise::CreateReply;
///
/// Transfer some bucks between players
///
/// Enter `/transfer <source> <recipient> <amount>`
/// ```
/// /transfer @John @Adam 50
/// ```
#[poise::command(
    slash_command,
    category = "Admin",
    hide_in_help,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn transfer(
    ctx: Context<'_>,
    #[description = "Who to remove from"] source: User,
    #[description = "Who to give to"] recipient: User,
    #[min = 1]
    #[description = "How much to transfer"]
    amount: i32,
) -> Result<(), Error> {
    if source.id == recipient.id {
        let reply = {
            CreateReply::default()
                .content("No action required")
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("No action required".into());
    }
    if source.bot || recipient.bot {
        let reply = {
            CreateReply::default()
                .content("You can't transfer money to or from bots..")
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("You can't afford to do that".into());
    }
    let user_id = source.id.to_string();
    let user_balance = ctx.data().db.get_balance(user_id.clone()).await?;
    if user_balance < amount {
        let reply = {
            CreateReply::default()
                .content(format!(
                    "They can't afford to do that!\n{}'s balance is only {} J-Buck(s)",
                    source, user_balance
                ))
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("can't afford to do that".into());
    }
    let recipient_id = recipient.id.to_string();
    let recipient_balance = ctx.data().db.get_balance(recipient_id.clone()).await?;
    ctx.data()
        .db
        .set_balance(user_id.clone(), user_balance - amount)
        .await?;
    ctx.data()
        .db
        .set_balance(recipient_id.clone(), recipient_balance + amount)
        .await?;

    let reply = {
        CreateReply::default().content(format!(
            "Removed {} J-Buck(s) from {} and gave it to {}",
            amount, source, recipient
        ))
    };
    ctx.send(reply).await?;
    Ok(())
}
