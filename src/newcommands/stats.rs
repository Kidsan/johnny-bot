use crate::{database::BalanceDatabase, Context, Error};
use poise::CreateReply;

///
/// Get some stats about the economy
///
/// Enter `/stats` to get some free J-Bucks every day!
/// ```
/// /stats
/// ```
#[poise::command(
    slash_command,
    category = "Admin",
    default_member_permissions = "ADMINISTRATOR",
    hide_in_help
)]
pub async fn stats(ctx: Context<'_>) -> Result<(), Error> {
    let total_economy = ctx.data().db.get_total().await?;
    let avg_balance = ctx.data().db.get_avg_balance().await?;
    let count_of_zero = ctx.data().db.get_zero_balance().await?;

    let message = format!(
        "Total economy: {}\nAverage balance: {}\nCount of zero balances: {}",
        total_economy, avg_balance, count_of_zero
    );

    let reply = CreateReply::default().content(message).ephemeral(true);
    ctx.send(reply).await?;

    Ok(())
}
