use crate::commands::robbingevent::get_discord_name;
use crate::{database::BalanceDatabase, database::LotteryDatabase, Context, Error};
use poise::CreateReply;

///
/// Commands relating to the lottery
///
/// Enter `/lottery info or /lottery tickets`
/// ```
/// /lottery info
/// ```
#[poise::command(slash_command, subcommands("info"))]
#[tracing::instrument(level = "info")]
pub async fn lottery(ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

///
/// Get information about the current lottery
///
/// Enter `/lottery info`
/// ```
/// /lottery info
/// ```
#[poise::command(slash_command)]
#[tracing::instrument(level = "info")]
pub async fn info(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;
    let data = ctx.data().db.get_bought_tickets().await.unwrap();
    let base_prize = { ctx.data().config.read().unwrap().lottery_base_prize };
    let price = { ctx.data().config.read().unwrap().lottery_ticket_price };

    let tickets_sold = data.iter().map(|(_, v)| v).sum::<i32>();
    let prize = (tickets_sold * (price - 1)) + base_prize;

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

    let mut player_names = std::collections::HashMap::new();

    for (user, _) in data.iter().take(10) {
        player_names.insert(user, get_discord_name(ctx, *user).await);
    }

    let info = format!(
        "> **Prize pool:** {} <:jbuck:1228663982462865450>\n> **Tickets sold:** {} :tickets:\n > **End:** <t:{}:R>",
        prize, tickets_sold, end.and_utc().timestamp(),
    );

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

    let price = { ctx.data().config.read().unwrap().lottery_ticket_price };

    if a.len() < data.len() {
        a.push(String::from(
            "> *Sorry, we don't have the technology to show more than 10 people*",
        ));
    }
    a.insert(0, String::from("> **CURRENT LOTTERY TICKET HOLDERS**"));
    if a.len() == 1 {
        a.push(String::from("> *No one has bought a ticket yet*"));
    }
    a.push(info);

    a.push(format!(
        "> Use ***/buy lottery*** to purchase a ticket for {} <:jbuck:1228663982462865450>",
        price
    ));

    let reply = CreateReply::default().content(format!("{}", a.join("\n")));
    ctx.send(reply).await?;
    Ok(())
}

///
/// Buy a lottery ticket
///
/// Enter `/buy lottery`
/// ```
/// /buy lottery
/// ```
#[poise::command(slash_command, rename = "lottery")]
pub async fn buylotteryticket(
    ctx: Context<'_>,
    #[description = "The amount of tickets to buy"]
    #[min = 1]
    #[max = 100]
    amount: Option<i32>,
) -> Result<(), Error> {
    let amount = amount.unwrap_or(1);
    let user_balance = ctx.data().db.get_balance(ctx.author().id.get()).await?;
    let base_prize = { ctx.data().config.read().unwrap().lottery_base_prize };
    let price = { ctx.data().config.read().unwrap().lottery_ticket_price };
    if price * amount > user_balance {
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
        .subtract_balances(vec![ctx.author().id.get()], price * amount)
        .await?;

    let owned_tickets = ctx
        .data()
        .db
        .bought_lottery_ticket(ctx.author().id.get(), amount)
        .await?;

    let prize = ctx
        .data()
        .db
        .get_bought_tickets()
        .await
        .unwrap()
        .iter()
        .map(|(_, x)| x * (price - 1))
        .sum::<i32>()
        + base_prize;

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
