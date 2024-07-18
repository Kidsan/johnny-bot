use crate::{database::BalanceDatabase, database::RoleDatabase, Context, Error};
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
#[tracing::instrument(level = "info")]
pub async fn daily(ctx: Context<'_>) -> Result<(), Error> {
    {
        if ctx
            .data()
            .active_checks
            .lock()
            .unwrap()
            .contains(&(ctx.author().id.get()))
        {
            return Err("You are already doing this!".to_string().into());
        }

        ctx.data()
            .active_checks
            .lock()
            .unwrap()
            .insert(ctx.author().id.get());
    }
    match daily_cooldown(ctx).await {
        Ok(_) => {}
        Err(e) => {
            ctx.data()
                .active_checks
                .lock()
                .unwrap()
                .remove(&(ctx.author().id.get()));
            return Err(e);
        }
    }
    let user_id = ctx.author().id.get();
    let amount = { ctx.data().rng.lock().unwrap().gen_range(5..=10) };
    let balance = { ctx.data().db.get_balance(ctx.author().id.get()).await? };

    let upper_limit = {
        let config = ctx.data().config.read().unwrap();
        if config.daily_upper_limit > 0 {
            Some(config.daily_upper_limit)
        } else {
            None
        }
    };

    if let Some(limit) = upper_limit {
        if balance >= limit {
            let msg = "You are too rich for handouts!";
            let reply = CreateReply::default().content(msg).ephemeral(true);
            ctx.send(reply).await?;
            ctx.data()
                .active_checks
                .lock()
                .unwrap()
                .remove(&(ctx.author().id.get()));
            return Err(msg.to_string().into());
        }
    }

    let interest = {
        let mp = { ctx.data().rng.lock().unwrap().gen_range(0.01..=0.03) };
        tracing::info!(
            "mp: {}, balance: {}, bonus: {}",
            mp,
            balance,
            (balance as f32 * mp) as i32
        );
        (balance as f32 * mp) as i32
    };

    let n = {
        let user = poise::serenity_prelude::UserId::new(ctx.author().id.into())
            .to_user(ctx)
            .await
            .unwrap();

        let guild_id = ctx.guild_id().unwrap();
        let guild = ctx.guild().unwrap().clone();

        let has = match guild.role_by_name("Nitro Dealers") {
            Some(role) => {
                let has_role = user.has_role(ctx, guild_id, role.id).await;
                has_role.unwrap()
            }
            None => {
                tracing::error!("Nitro Dealers role not found");
                false
            }
        };

        let mut v = 0;
        if has {
            let mp = { ctx.data().rng.lock().unwrap().gen_range(1.5..=2.0) };
            let bonus = ((amount as f32 + interest as f32) * mp) as i32;
            v = bonus - amount - interest
        }
        v
    };

    let crown_interest = {
        let has = if let Some(u) = ctx
            .data()
            .db
            .get_unique_role_holder(ctx.data().crown_role_id)
            .await?
        {
            u.user_id == user_id
        } else {
            false
        };

        let mut v = 0;
        if has {
            let mp = { ctx.data().rng.lock().unwrap().gen_range(1.5..=2.0) };
            let bonus = ((amount as f32 + interest as f32 + n as f32) * mp) as i32;
            v = bonus - amount - interest - n;
        }
        v
    };

    ctx.data()
        .db
        .award_balances(vec![user_id], amount + interest + n + crown_interest)
        .await?;
    ctx.data().db.did_daily(user_id).await?;
    ctx.data()
        .active_checks
        .lock()
        .unwrap()
        .remove(&(ctx.author().id.get()));
    let reply = {
        let msg = format!(
            "You got **{}** <:jbuck:1228663982462865450>!{}{}{}",
            amount,
            if interest > 0 {
                format!("\n**+{}** <:jbuck:1228663982462865450> interest!", interest)
            } else {
                "".to_string()
            },
            if n > 0 {
                format!("\n+**{}** <:jbuck:1228663982462865450> booster bonus!", n)
            } else {
                "".to_string()
            },
            if crown_interest > 0 {
                format!(
                    "\n+**{}** <:jbuck:1228663982462865450> crown holder bonus!",
                    crown_interest
                )
            } else {
                "".to_string()
            }
        );
        CreateReply::default().content(msg)
    };
    ctx.send(reply).await?;
    Ok(())
}

async fn daily_cooldown(ctx: Context<'_>) -> Result<(), Error> {
    if let Some(last_daily) = ctx.data().db.get_last_daily(ctx.author().id.get()).await? {
        let today = chrono::Utc::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap();

        let tomorrow = today + chrono::Duration::days(1);
        if last_daily.naive_utc() > today {
            let ts = tomorrow.and_utc().timestamp();

            let reply = {
                CreateReply::default()
                    .content(format!(
                        "You can only do this once per day! Try again <t:{}:R>.",
                        ts
                    ))
                    .ephemeral(true)
            };
            ctx.send(reply).await?;
            return Err("You can only do this once per day.".to_string().into());
        }
    }
    Ok(())
}
