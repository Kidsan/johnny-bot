use crate::database::BalanceDatabase;
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
    let user_id = ctx.author().id.to_string();
    let response = ctx.data().db.get_balance(user_id).await?;
    let reply = {
        poise::CreateReply::default()
            .content(format!("{} has {} J-Buck(s)!", ctx.author(), response,))
            .ephemeral(true)
    };
    ctx.send(reply).await?;
    Ok(())
}
