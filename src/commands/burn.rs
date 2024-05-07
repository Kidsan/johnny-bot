use crate::{database::BalanceDatabase, Context, Error};
use poise::CreateReply;

///
/// Bury some money
///
/// Enter `/bury <amount>` to bury some cash in a display of power
/// ```
/// /bury 10
/// ```
#[poise::command(slash_command)]
pub async fn bury(
    ctx: Context<'_>,
    #[description = "Amount to bury"]
    #[min = 1]
    amount: i32,
) -> Result<(), Error> {
    let balance = ctx
        .data()
        .db
        .get_balance(ctx.author().id.to_string())
        .await?;
    if amount > balance {
        let reply = {
            CreateReply::default()
                .content(format!(
                    "You can't afford to bury {}. You only have {} <:jbuck:1228663982462865450>!",
                    amount, balance
                ))
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("Not enough money".into());
    }
    ctx.data()
        .db
        .subtract_balances(vec![ctx.author().id.to_string()], amount)
        .await?;
    ctx.data()
        .db
        .bury_balance(ctx.author().id.to_string(), amount)
        .await?;
    let reply = {
        CreateReply::default().content(format!(
            "<:dogehehe:1228284291251703900> {} buried {} <:jbuck:1228663982462865450>!",
            ctx.author(),
            amount
        ))
    };
    ctx.send(reply).await?;
    Ok(())
}
