use crate::{database::BalanceDatabase, Context, Error};
use poise::CreateReply;

///
/// Burn some money
///
/// Enter `/burn <amount>` to burn some cash in a display of power
/// ```
/// /burn 10
/// ```
#[poise::command(slash_command)]
pub async fn burn(
    ctx: Context<'_>,
    #[description = "Amount to burn"]
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
                    "You can't afford to burn {}. You only have {} :dollar:!",
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
    let reply = {
        CreateReply::default().content(format!(
            "<a:dogeLaughlit:1160530388008050709>** {} burned {} :dollar:! **",
            ctx.author(),
            amount
        ))
    };
    ctx.send(reply).await?;
    Ok(())
}
