use crate::database::BalanceDatabase;
use crate::{Context, Error};
use poise::serenity_prelude::User;
use poise::CreateReply;

use super::rockpaperscissors::award_role_holder;

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
    let sender_balance = ctx
        .data()
        .db
        .get_balance(ctx.author().id.get().try_into().unwrap())
        .await?;
    let recipient_id: i64 = recipient.id.get().try_into().unwrap();
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

    db.subtract_balances(vec![sender.clone()], amount).await?;
    db.award_balances(vec![recipient_id], amount - tax).await?;

    let tax_msg = if let Some(user) = award_role_holder(ctx, tax).await? {
        format!(
            "-{} <:jbuck:1228663982462865450> to <@{}> (Crown's Tax)",
            tax, user
        )
    } else {
        "".to_string()
    };
    let reply = {
        CreateReply::default()
            .content(format!(
                "{} sent {} <:jbuck:1228663982462865450> to {}!\n{}",
                ctx.author(),
                amount,
                recipient,
                tax_msg,
            ))
            .allowed_mentions(
                poise::serenity_prelude::CreateAllowedMentions::new()
                    .users(vec![ctx.author(), &recipient]),
            )
    };
    ctx.send(reply).await?;
    Ok(())
}
