use crate::database::BalanceDatabase;
use crate::{Context, Error};
use poise::serenity_prelude::User;
use poise::CreateReply;
///
/// award bucks to a player
///
/// Enter `/award <player> <amount>`
/// ```
/// /award @John 50
/// ```
#[poise::command(
    slash_command,
    category = "Admin",
    hide_in_help,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn award(
    ctx: Context<'_>,
    #[description = "Who to award"] user: User,
    #[min = 1]
    #[description = "How much to award"]
    amount: i32,
    #[description = "Show that you invoked the command?"] show_caller: Option<bool>,
    #[description = "Reason for award"] reason: Option<String>,
) -> Result<(), Error> {
    if user.bot {
        let reply = {
            CreateReply::default()
                .content("You can't award bots..")
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("You can't afford to do that".into());
    }
    let user_id = user.id.to_string();
    let user_balance = ctx
        .data()
        .db
        .get_balance(user.id.get().try_into().unwrap())
        .await?;
    ctx.data()
        .db
        .set_balance(user_id.clone(), user_balance + amount)
        .await?;

    // if show_caller is true, send as a reply
    let msg = match reason {
        Some(m) => format!(
            "{} was awarded {} <:jbuck:1228663982462865450>!\nReason: \"*{}*\"",
            user, amount, m
        ),
        None => format!(
            "{} was awarded {} <:jbuck:1228663982462865450>!",
            user, amount
        ),
    };
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
