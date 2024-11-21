use std::fmt::Display;

use crate::database::BalanceDatabase;
use crate::{Context, Error};
use poise::serenity_prelude::User;
use poise::CreateReply;

use super::rockpaperscissors::award_role_holder;

#[derive(poise::ChoiceParameter, Clone, Debug)]
enum WhatToGive {
    Bucks,
    Bones,
}

impl Display for WhatToGive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                WhatToGive::Bucks => "<:jbuck:1228663982462865450>",
                WhatToGive::Bones => ":bone:",
            }
        )
    }
}

///
/// Give bucks or bones to another player
///
/// Enter `/give <recipient> <amount> [what]`
/// ```
/// /give @John 50 Bucks
/// /give @John 5 Bones
/// ```
#[poise::command(slash_command)]
pub async fn give(
    ctx: Context<'_>,
    #[description = "Who to send to"] recipient: User,
    #[min = 1]
    #[max = 1000]
    #[description = "How much to send"]
    amount: i32,
    #[description = "What to give?"] what: Option<WhatToGive>,
) -> Result<(), Error> {
    if recipient.id.get() == ctx.author().id.get() {
        let reply = {
            CreateReply::default()
                .content("Don't send to yourself..")
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("You can't send to yourself".into());
    }
    if recipient.bot {
        let reply = {
            CreateReply::default()
                .content("You can't send to bots..")
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("You can't send to bots.".into());
    }

    let currency = match what {
        Some(b) => b,
        None => WhatToGive::Bucks,
    };
    let sender = ctx.author().id.get();
    let db = &ctx.data().db;
    let sender_balance = match currency {
        WhatToGive::Bucks => ctx.data().db.get_balance(sender).await?,
        WhatToGive::Bones => ctx.data().db.get_bones(sender).await?,
    };
    let recipient_id = recipient.id.get();
    if sender_balance < amount {
        let reply = {
            CreateReply::default()
                .content(format!(
                    "You can't afford to do that!\nYour balance is only {} {}",
                    sender_balance, currency,
                ))
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("You can't afford to do that".into());
    }

    let tax = match currency {
        WhatToGive::Bucks => amount as f32 * 0.02,
        WhatToGive::Bones => 0.0,
    };
    // round tax up to the nearest integer
    let tax = tax.ceil() as i32;

    match currency {
        WhatToGive::Bucks => {
            db.subtract_balances(vec![sender], amount).await?;
            db.award_balances(vec![recipient_id], amount - tax).await?;
        }
        WhatToGive::Bones => {
            db.remove_bones(sender, amount).await?;
            db.add_bones(recipient_id, amount).await?;
        }
    };

    let tax_msg = if let Some(user) = award_role_holder(ctx, tax).await? {
        format!(
            "-{} <:jbuck:1228663982462865450> to <@{}> (Crown's Tax)",
            tax, user
        )
    } else {
        "".to_string()
    };
    let reply = {
        CreateReply::default()
            .content(format!(
                "{} sent {} {} to {}!\n{}",
                ctx.author(),
                amount,
                currency,
                recipient,
                tax_msg,
            ))
            .allowed_mentions(
                poise::serenity_prelude::CreateAllowedMentions::new()
                    .users(vec![ctx.author(), &recipient]),
            )
    };
    ctx.send(reply).await?;
    Ok(())
}
