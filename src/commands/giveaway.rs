use crate::database::BalanceDatabase;
use crate::{Context, Error};
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::User;
use poise::CreateReply;
use std::time::{self, SystemTime, UNIX_EPOCH};

fn enter_giveaway_button() -> serenity::CreateButton {
    serenity::CreateButton::new("Yes Please")
        .label("Yes Please")
        .style(poise::serenity_prelude::ButtonStyle::Primary)
}

///
/// do a giveaway
///
/// Enter `/giveaway`
/// ```
/// /giveaway
/// ```
#[poise::command(
    slash_command,
    category = "Admin",
    hide_in_help,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn giveaway(
    ctx: Context<'_>,
    #[description = "Message to include in giveaway"] message: String,
    #[description = "How long the giveaway lasts in seconds"] length: u64,
    #[description = "How much to award"] amount: i32,
) -> Result<(), Error> {
    let reply = {
        poise::CreateReply::default()
            .content("Success!")
            .ephemeral(true)
    };
    ctx.send(reply).await?;
    let played = std::sync::Mutex::new(std::collections::HashSet::new());
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let components = vec![serenity::CreateActionRow::Buttons(vec![
        enter_giveaway_button(),
    ])];
    let reply = {
        serenity::CreateMessage::default()
            .content(format!(
                "> ### <:jbuck:1228663982462865450> Giveaway time!\n> **{}**\n> **Ends: **<t:{}:R>",
                message,
                now + length
            ))
            .components(components.clone())
    };

    let mut a = ctx.channel_id().send_message(ctx, reply).await?;
    let id = a.id;
    while let Some(mci) = serenity::ComponentInteractionCollector::new(ctx)
        .channel_id(ctx.channel_id())
        .custom_ids(vec!["Yes Please".to_string()])
        .message_id(id)
        .timeout(std::time::Duration::from_secs(
            (now + length - 1) - SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        ))
        .await
    {
        if played.lock().unwrap().contains(&mci.user.id) {
            mci.create_response(
                ctx,
                serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::new()
                        .content("Nice try, but you can only enter once!")
                        .ephemeral(true),
                ),
            )
            .await?;
            continue;
        }

        ctx.data()
            .db
            .award_balances(vec![mci.user.id.into()], amount)
            .await
            .unwrap();
        played.lock().unwrap().insert(mci.user.id);
        mci.create_response(
            ctx,
            serenity::CreateInteractionResponse::Message(
                serenity::CreateInteractionResponseMessage::new().content(format!(
                    "Congratulations <@{}>! You got {} <:jbuck:1228663982462865450>!",
                    mci.user.id, amount
                )),
            ),
        )
        .await
        .unwrap();
    }
    a.edit(
            ctx,
            serenity::EditMessage::new()
            .content(format!(
                "> ### <:jbuck:1228663982462865450> Giveaway time!\n> **{}**\n> **Ended: **<t:{}:R>",
                message, now + length
            ))
            .components(vec![]),
        )
        .await?;

    Ok(())
}
