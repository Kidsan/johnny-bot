use crate::database::BalanceDatabase;
use crate::{Context, Error};
use poise::serenity_prelude::User;
use poise::CreateReply;
///
/// Fine a player
///
/// Enter `/fine <player> <amount>`
/// ```
/// /fine @John 50
/// ```
///
#[poise::command(
    slash_command,
    category = "Admin",
    hide_in_help,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn fine(
    ctx: Context<'_>,
    #[description = "Who to fine"] user: User,
    #[min = 1]
    #[description = "How much to fine them"]
    amount: i32,
    #[description = "Show that you invoked the command?"] show_caller: Option<bool>,
    #[description = "Reason for fine"] reason: Option<String>,
) -> Result<(), Error> {
    let user_id = user.id.to_string();
    let user_balance = ctx.data().db.get_balance(user.id.get()).await?;
    if user_balance < amount {
        let reply = {
            CreateReply::default()
                .content(format!(
                    "They can't afford to do that!\n{}'s balance is only {} J-Bucks",
                    user, user_balance
                ))
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("Can't afford to do that".into());
    }
    ctx.data()
        .db
        .subtract_balances(vec![user_id.parse().unwrap()], amount)
        .await?;

    let msg = match reason {
        Some(r) => format!(
            "{} was fined {} <:jbuck:1228663982462865450>!\nReason: \"*{}*\"",
            user, amount, r
        ),
        None => format!(
            "{} was fined {} <:jbuck:1228663982462865450>!",
            user, amount
        ),
    };

    // if show_caller is true, send as a reply
    match show_caller {
        Some(true) => {
            let reply = { CreateReply::default().content(msg) };
            ctx.send(reply).await?;
        }
        _ => {
            // acknowledge the invocation
            ctx.send(CreateReply::default().content("success").ephemeral(true))
                .await?;
            ctx.channel_id().say(ctx, msg).await?;
        }
    }
    Ok(())
}
