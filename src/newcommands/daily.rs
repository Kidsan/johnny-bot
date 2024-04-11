use crate::{database::BalanceDatabase, Context, Error};
use poise::CreateReply;
use rand::Rng;

///
/// Claim your daily J-Bucks
///
/// Enter `/daily` to get some free J-Bucks every day!
/// ```
/// /daily
/// ```
#[poise::command(slash_command)]
pub async fn daily(ctx: Context<'_>) -> Result<(), Error> {
    let daily_timer = std::time::Duration::from_secs(86400);
    let time_since = {
        let last_daily = ctx
            .data()
            .db
            .get_last_daily(ctx.author().id.to_string())
            .await?;

        let diff = chrono::Utc::now() - last_daily;
        diff.to_std().unwrap()
    };
    let time_remaining = match daily_timer.checked_sub(time_since) {
        Some(time) => time,
        None => daily_timer,
    };
    if time_remaining < daily_timer {
        if time_remaining < std::time::Duration::from_secs(60) {
            // show in seconds if less than a minute
            let reply = {
                CreateReply::default()
                    .content(format!(
                        "You can only do this once per day! Try again in {} seconds.",
                        time_remaining.as_secs()
                    ))
                    .ephemeral(true)
            };
            ctx.send(reply).await?;
            return Err(format!("Please wait {} seconds", time_since.as_secs()).into());
        } else if time_remaining < std::time::Duration::from_secs(3600) {
            // show in minutes if less than an hour
            let reply = {
                CreateReply::default()
                    .content(format!(
                        "You can only do this once per day! Try again in {} minutes.",
                        time_remaining.as_secs() / 60
                    ))
                    .ephemeral(true)
            };
            ctx.send(reply).await?;
            return Err(format!("Please wait {} minutes", time_since.as_secs() / 60).into());
        }

        let reply = {
            CreateReply::default()
                .content(format!(
                    "You can only do this once per day! Try again in {} hours.",
                    time_remaining.as_secs() / 3600
                ))
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err(format!(
            "You can only do this once per day! Try again in {} hours.",
            time_remaining.as_secs() / 3600
        )
        .into());
    }
    let user_id = ctx.author().id.to_string();
    let amount = { ctx.data().rng.lock().unwrap().gen_range(5..=10) };
    ctx.data()
        .db
        .award_balances(vec![user_id.clone()], amount)
        .await?;
    ctx.data().db.did_daily(user_id).await?;
    let reply = { CreateReply::default().content(format!("You got {} :dollar:!", amount)) };
    ctx.send(reply).await?;
    Ok(())
}
