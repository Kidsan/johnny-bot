use crate::database::BalanceDatabase;
use crate::{Context, Error};
use poise::serenity_prelude as serenity;
use rand::seq::SliceRandom;
use std::time::{SystemTime, UNIX_EPOCH};

fn option_button(text: String) -> serenity::CreateButton {
    serenity::CreateButton::new(text.clone())
        .label(text)
        .style(poise::serenity_prelude::ButtonStyle::Primary)
}

#[allow(clippy::too_many_arguments)]
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
    #[description = "How long the giveaway lasts in minutes"]
    #[min = 1]
    length_minutes: u64,
    #[min = 1]
    #[description = "How much to award"]
    amount: i32,
    #[description = "the winning option"] option: String,
    #[description = "option two"] option2: Option<String>,
    #[description = "option three"] option3: Option<String>,
    #[description = "option four"] option4: Option<String>,
) -> Result<(), Error> {
    let reply = {
        poise::CreateReply::default()
            .content("Success!")
            .ephemeral(true)
    };
    ctx.send(reply).await?;

    let played = std::sync::Mutex::new(std::collections::HashSet::new());
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let mut options = [
        option.clone(),
        option2.unwrap_or_default(),
        option3.unwrap_or_default(),
        option4.unwrap_or_default(),
    ];
    {
        let mut rng = rand::thread_rng();
        options.shuffle(&mut rng);
        options.shuffle(&mut rng);
        options.shuffle(&mut rng);
    }

    let buttons = options
        .iter()
        .filter(|p| !p.is_empty())
        .map(|option| option_button(option.to_string()))
        .collect();
    let components = vec![serenity::CreateActionRow::Buttons(buttons)];

    let reply = {
        serenity::CreateMessage::default()
            .content(format!(
                "> ### <:jbuck:1228663982462865450> Giveaway time!\n> **{}**\n> **Ends: **<t:{}:R>",
                message,
                now + (length_minutes * 60)
            ))
            .components(components.clone())
    };

    let mut a = ctx.channel_id().send_message(ctx, reply).await?;
    let id = a.id;
    while let Some(mci) = serenity::ComponentInteractionCollector::new(ctx)
        .channel_id(ctx.channel_id())
        .custom_ids(options.iter().filter(|p| !p.is_empty()).cloned().collect())
        .message_id(id)
        .timeout(std::time::Duration::from_secs(
            (now + (length_minutes * 60) - 1)
                - SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
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

        played.lock().unwrap().insert(mci.user.id);
        if mci.data.custom_id == option {
            ctx.data()
                .db
                .award_balances(vec![mci.user.id.into()], amount)
                .await
                .unwrap();
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
        } else {
            mci.create_response(
                ctx,
                serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::new()
                        .content("Better luck next time!")
                        .ephemeral(true),
                ),
            )
            .await?;
        }
    }
    a.edit(
            ctx,
            serenity::EditMessage::new()
            .content(format!(
                "> ### <:jbuck:1228663982462865450> Giveaway time!\n> **{}**\n> **Ended: **<t:{}:R>\n> **Players: ** {}",
                message, now + (length_minutes *60), played.lock().unwrap().len()
            ))
            .components(vec![]),
        )
        .await?;

    Ok(())
}
