use crate::database::{BalanceDatabase, LotteryDatabase};
use crate::{Context, Error};

///
/// Check your balance
///
/// Enter `/balance` to check
/// ```
/// /balance
/// ```
#[poise::command(slash_command)]
pub async fn balance(ctx: Context<'_>) -> Result<(), Error> {
    let response = ctx.data().db.get_balance(ctx.author().id.get()).await?;
    let crown_time = ctx.data().db.get_crown_time(ctx.author().id.get()).await?;
    let lottery_tickets = ctx
        .data()
        .db
        .get_user_tickets(ctx.author().id.get())
        .await?;

    let hours = crown_time.1.trunc() as i32;
    let minutes = (((crown_time.1.fract() * 100.0).round() / 100.0) * 60.0) as i32;

    let response = format!(
        "> **{}'s Balance** \n> \n> **Balance:** {} <:jbuck:1228663982462865450>\n> **Lottery Tickets:** {} :tickets:\n> **Crown Time**: {:0>2}:{:0>2} :clock1:",
        ctx.author(),
        response,
        lottery_tickets,
        hours, minutes
    );
    let reply = {
        poise::CreateReply::default()
            .content(response)
            .ephemeral(true)
    };
    ctx.send(reply).await?;
    Ok(())
}
