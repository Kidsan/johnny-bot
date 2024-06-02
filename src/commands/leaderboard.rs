use crate::{commands::robbingevent::get_discord_name, database::BalanceDatabase, Context, Error};
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
            map.insert(player.clone(), format!("@{}", name));
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
