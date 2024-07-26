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

    let response = format!(
        "**Balance data for {}:\n\nBalance:** {} <:jbuck:1228663982462865450>\n**Lottery Tickets:** {}\n**Crown Time**: {}",
        ctx.author(),
        response,
        lottery_tickets,
        crown_time.1
    );
    let reply = {
        poise::CreateReply::default()
            .content(response)
            .ephemeral(true)
    };
    ctx.send(reply).await?;
    Ok(())
}
