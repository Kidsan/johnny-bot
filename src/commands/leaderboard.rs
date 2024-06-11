use crate::{
    commands::robbingevent::get_discord_name,
    database::{BalanceDatabase, RoleDatabase},
    Context, Error,
};
use poise::{serenity_prelude::CreateAllowedMentions, CreateReply};

///
/// View Leaderboard
///
/// Enter `/leaderboard` to view
/// ```
/// /leaderboard
/// ```
#[poise::command(slash_command)]
pub async fn leaderboard(ctx: Context<'_>) -> Result<(), Error> {
    let balances = ctx.data().db.get_leaderboard().await?;

    let named_players = {
        let mut map = std::collections::HashMap::new();
        for (player, _) in balances.clone() {
            let name = get_discord_name(ctx, &player).await;
            map.insert(player.clone(), name);
        }
        map
    };

    let top = balances
        .iter()
        .map(|(k, v)| (named_players.get(k).unwrap(), v))
        .enumerate()
        .map(|(i, (k, v))| {
            if i == 0 {
                return format!("> <:jbuck:1228663982462865450> **{}** - **{}**", v, k);
            }
            format!("> <:jbuck:1228663982462865450> **{}** - {}", v, k)
        })
        .collect::<Vec<_>>()
        .join("\n");
    if top.is_empty() {
        ctx.say("Nobody has any J-Bucks yet!").await?;
        return Ok(());
    }

    let reply = {
        CreateReply::default()
            .content(format!(
                "> ### Top {} <:jbuck:1228663982462865450> Holders\n> \n{}\n> ***Keep gambling.***",
                balances.len(),
                top
            ))
            .allowed_mentions(CreateAllowedMentions::new().empty_users())
    };

    ctx.send(reply).await?;
    Ok(())
}

///
/// View Crown Leaderboard
///
/// Enter `/crownleaderboard` to view
/// ```
/// /crownleaderboard
/// ```
#[poise::command(slash_command)]
pub async fn crownleaderboard(ctx: Context<'_>) -> Result<(), Error> {
    let balances = ctx.data().db.get_crown_leaderboard().await?;

    let crown_holder = ctx
        .data()
        .db
        .get_unique_role_holder(ctx.data().crown_role_id)
        .await?;

    let named_players = {
        let mut map = std::collections::HashMap::new();
        for (player, _) in balances.clone() {
            let name = get_discord_name(ctx, &player.to_string()).await;
            map.insert(player, name);
        }
        map
    };

    let mut top = balances
        .iter()
        .map(|(k, v)| {
            if let Some(crown) = &crown_holder {
                if k.to_string() == crown.user_id {
                    let now = chrono::Utc::now();
                    let bought = crown.purchased;
                    let time_since_purchase = now - bought;
                    let a = v + time_since_purchase.num_minutes() as f32 / 60.0;
                    return (k, a);
                }
            }
            (k, *v)
        })
        .collect::<Vec<_>>();
    top.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    let crown_holder_name = {
        if let Some(crown) = &crown_holder {
            get_discord_name(ctx, &crown.user_id).await
        } else {
            "".to_string()
        }
    };

    let mut top_text = top
        .iter()
        .enumerate()
        .map(|(i, (k, v))| {
            let name = named_players.get(k).unwrap();
            let hours = v.trunc() as i32;
            let minutes = (v.fract() * 60.0) as i32;
            if let Some(crown) = &crown_holder {
                if k.to_string() == crown.user_id {
                    return format!(
                        "> :crown: **{:0>2}:{:0>2}** - **{}**",
                        hours, minutes, crown_holder_name
                    );
                }
            } else if i == 0 {
                return format!(
                    "> :clock{}: **{:0>2}:{:0>2}** - **{}**",
                    i + 1,
                    hours,
                    minutes,
                    name
                );
            }
            format!(
                "> :clock{}: **{:0>2}:{:0>2}** - {}",
                i + 1,
                hours,
                minutes,
                name
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    if let Some(crown) = &crown_holder {
        if top_text.is_empty() {
            let now = chrono::Utc::now();
            let bought = crown.purchased;
            let time_since_purchase = now - bought;
            let a = time_since_purchase.num_minutes() as f32 / 60.0;
            top_text = format!(
                "> :clock1: **{:.2} Hours** - **{}**",
                a,
                get_discord_name(ctx, &crown.user_id).await
            );
        }
    }

    if top_text.is_empty() {
        ctx.say("Nobody has had the crown yet!").await?;
        return Ok(());
    }

    let reply = {
        CreateReply::default()
            .content(format!("> ### Crown Time Leaderboard \n> \n{}\n", top_text))
            .allowed_mentions(CreateAllowedMentions::new().empty_users())
    };

    ctx.send(reply).await?;
    Ok(())
}
