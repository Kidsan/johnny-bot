use crate::commands::robbingevent::week_bounds;
use crate::database::{BalanceDatabase, LotteryDatabase, RobberyDatabase};
use crate::{Context, Error};
use chrono::Datelike;

///
/// Check your balance
///
/// Enter `/balance` to check
/// ```
/// /balance
/// ```
#[poise::command(slash_command)]
pub async fn balance(ctx: Context<'_>) -> Result<(), Error> {
    let response = ctx.data().db.get_balance(ctx.author().id.get()).await?;
    let crown_time = ctx.data().db.get_crown_time(ctx.author().id.get()).await?;
    let lottery_tickets = ctx
        .data()
        .db
        .get_user_tickets(ctx.author().id.get())
        .await?;

    let robbery_status: String;
    {
        let guild_id = ctx.guild_id().unwrap();
        let last_bought_robbery = ctx
            .data()
            .db
            .get_last_bought_robbery_two(ctx.author().id.get())
            .await?;

        let a = ctx
            .serenity_context()
            .http
            .get_member(guild_id, ctx.author().id)
            .await?;

        let nitro_role = match ctx.guild().unwrap().role_by_name("Nitro Dealers") {
            Some(x) => x.clone(),
            None => ctx
                .guild()
                .unwrap()
                .roles
                .get(&poise::serenity_prelude::RoleId::new(1236716462266122250))
                .unwrap()
                .clone(),
        };

        let has = a
            .roles
            .iter()
            .any(|&x| x == poise::serenity_prelude::RoleId::new(1236716462266122250))
            || ctx
                .author()
                .has_role(ctx, guild_id, nitro_role)
                .await
                .unwrap();

        if !has {
            robbery_status = "License Needed".to_string();
        } else if last_bought_robbery.is_none() {
            robbery_status = "Ready".to_string();
        } else {
            let week_number = chrono::Utc::now().date_naive().iso_week().week();
            let (start, _end) = week_bounds(week_number);
            if last_bought_robbery.unwrap().naive_utc() > start.into() {
                robbery_status = "Used".to_string();
            } else {
                robbery_status = "Ready".to_string();
            }
        }
    };

    // check if last bought robbery is after Monday 00:00

    let hours = crown_time.1.trunc() as i32;
    let minutes = (((crown_time.1.fract() * 100.0).round() / 100.0) * 60.0) as i32;

    let response = format!(
        "> **{}'s Balance** \n> \n> **Balance:** {} <:jbuck:1228663982462865450>\n> **Lottery Tickets:** {} :tickets:\n> **Crown Time**: {:0>2}:{:0>2} :clock1:\n> **Robbery Status**: {} :moneybag:",
        ctx.author(),
        response,
        lottery_tickets,
        hours, minutes,
        robbery_status
    );
    let reply = {
        poise::CreateReply::default()
            .content(response)
            .ephemeral(true)
    };
    ctx.send(reply).await?;
    Ok(())
}
