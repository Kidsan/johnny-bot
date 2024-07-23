use crate::commands::robbingevent::get_discord_name;
use crate::{database::BalanceDatabase, database::LotteryDatabase, Context, Error};
use poise::CreateReply;

#[poise::command(slash_command, subcommands("info", "tickets", "buy"))]
#[tracing::instrument(level = "info")]
pub async fn lottery(ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

///
/// Get information about the current lottery.
///
/// Enter `/lottery info`
/// ```
/// /lottery info
/// ```
#[poise::command(slash_command)]
#[tracing::instrument(level = "info")]
pub async fn info(ctx: Context<'_>) -> Result<(), Error> {
    let data = ctx.data().db.get_bought_tickets().await.unwrap();

    let tickets_sold = data.iter().map(|(_, v)| v).sum::<i32>();
    let prize = (tickets_sold * 5) + 10;

    // ends at the next 18:00 UTC
    let mut end = chrono::Utc::now()
        .naive_utc()
        .date()
        .and_hms_opt(18, 0, 0)
        .unwrap();

    let now = chrono::Utc::now().naive_utc();
    if end.lt(&now) {
        end += chrono::Duration::days(1);
    }

    let reply = CreateReply::default().content(format!(
        "> **LOTTERY STATUS**\n> **Prize pool:** {} <:jbuck:1228663982462865450>\n> **Tickets sold:** {} :tickets:\n > **End:** <t:{}:R>\n> Use ***/lottery buy*** to purchase a ticket for 5 <:jbuck:1228663982462865450>",
        prize, tickets_sold, end.and_utc().timestamp()
    ));
    ctx.send(reply).await?;
    Ok(())
}

///
/// Get information about the current lottery ticket holders
///
/// Enter `/lottery tickets`
/// ```
/// /lottery tickets
/// ```
#[poise::command(slash_command)]
#[tracing::instrument(level = "info")]
pub async fn tickets(ctx: Context<'_>) -> Result<(), Error> {
    let data = ctx.data().db.get_bought_tickets().await.unwrap();

    let mut player_names = std::collections::HashMap::new();

    for (user, _) in data.iter().take(10) {
        player_names.insert(user, get_discord_name(ctx, *user).await);
    }

    let mut a = data
        .iter()
        .take(10)
        .map(|(user, tix)| {
            format!(
                "> :tickets: {} - {}",
                tix,
                player_names.get(user).unwrap_or(&format!("<@{}>", user))
            )
        })
        .collect::<Vec<String>>();

    let total = data.iter().map(|(_, v)| v).sum::<i32>();

    if a.len() < data.len() {
        a.push(String::from(
            "> *Sorry, we don't have the technology to show more than 10 people*",
        ));
    }
    a.insert(0, String::from("> **CURRENT LOTTERY TICKET HOLDERS**"));
    if a.len() == 1 {
        a.push(String::from("> *No one has bought a ticket yet*"));
    }
    a.push(format!("> **Total sold:** {} :tickets:", total));

    a.push(String::from(
        "> Use ***/lottery buy*** to purchase a ticket for 5 <:jbuck:1228663982462865450>",
    ));

    let reply = CreateReply::default()
        .content(a.join("\n"))
        .allowed_mentions(poise::serenity_prelude::CreateAllowedMentions::new().empty_users());
    ctx.send(reply).await?;
    Ok(())
}

///
/// Buy a lottery ticket for 5 J-Bucks
///
/// Enter `/lottery buy`
/// ```
/// /lottery buy
/// ```
#[poise::command(slash_command)]
pub async fn buy(ctx: Context<'_>) -> Result<(), Error> {
    let user_balance = ctx.data().db.get_balance(ctx.author().id.get()).await?;
    if 5 > user_balance {
        let reply = {
            CreateReply::default()
                .content(format!(
                    "You can't afford to do that!\nYour balance is only {} J-Buck(s)",
                    user_balance
                ))
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("can't afford to do that".into());
    }

    ctx.data()
        .db
        .subtract_balances(vec![ctx.author().id.get()], 5)
        .await?;

    let owned_tickets = ctx
        .data()
        .db
        .bought_lottery_ticket(ctx.author().id.get())
        .await?;

    let prize = ctx
        .data()
        .db
        .get_bought_tickets()
        .await
        .unwrap()
        .iter()
        .map(|(_, x)| x * 5)
        .sum::<i32>()
        + 10;

    let reply = {
        CreateReply::default()
                .content(format!(
                    "> **<@{}> purchased a lottery ticket!**\n> They have a total of {} :tickets:\n> Prize pool increased to {} <:jbuck:1228663982462865450>",
                    ctx.author().id.get(), owned_tickets, prize
                ))
    };

    ctx.send(reply).await?;

    Ok(())
}
