use crate::database::BalanceDatabase;
use crate::{Context, Error};
use poise::serenity_prelude::User;
use poise::CreateReply;

///
/// Give some bucks to another player
///
/// Enter `/give <recipient> <amount>`
/// ```
/// /give @John 50
/// ```
#[poise::command(slash_command)]
pub async fn give(
    ctx: Context<'_>,
    #[description = "Who to send to"] recipient: User,
    #[min = 1]
    #[description = "How much to send"]
    amount: i32,
) -> Result<(), Error> {
    if recipient.id.to_string() == ctx.author().id.to_string() {
        let reply = {
            CreateReply::default()
                .content("Don't send money to yourself..")
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("You can't afford to do that".into());
    }
    if recipient.bot {
        let reply = {
            CreateReply::default()
                .content("You can't send money to bots..")
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("You can't afford to do that".into());
    }
    let sender = ctx.author().id.to_string();
    let db = &ctx.data().db;
    let sender_balance = ctx.data().db.get_balance(sender.clone()).await?;
    let recipient_id = recipient.id.to_string();
    let recipient_balance = ctx.data().db.get_balance(recipient_id.clone()).await?;
    if sender_balance < amount {
        let reply = {
            CreateReply::default()
                .content(format!(
                    "You can't afford to do that!\nYour balance is only {} J-Buck(s)",
                    sender_balance
                ))
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("You can't afford to do that".into());
    }

    let tax = amount as f32 * 0.02;
    // round tax up to the nearest integer
    let tax = tax.ceil() as i32;

    db.set_balance(sender.clone(), sender_balance - amount)
        .await?;
    db.set_balance(recipient_id.clone(), recipient_balance + (amount - tax))
        .await?;
    let reply = {
        CreateReply::default().content(format!(
            "{} sent {} <:jbuck:1228663982462865450> to {}!\n -{} <:jbuck:1228663982462865450> Johnny's work fee.",
            ctx.author(),
            amount,
            recipient,
            tax,
        ))
    };
    ctx.send(reply).await?;
    Ok(())
}
