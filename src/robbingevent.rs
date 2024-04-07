use crate::{database::BalanceDatabase, Context, Error};
use poise::CreateReply;
use rand::seq::SliceRandom;
use serenity::all::{
    ComponentInteractionCollector, CreateActionRow, CreateAllowedMentions, CreateButton,
    CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage,
};
use std::collections::{HashMap, HashSet};
///
/// Start a coin gamble
///
/// Enter `/gamble <amount>`
/// ```
/// /coingamble 10
/// ```
#[poise::command(slash_command)]
pub async fn robbingevent(ctx: Context<'_>) -> Result<(), Error> {
    let reply = {
        poise::CreateReply::default()
            .content("Success!")
            .ephemeral(true)
    };
    ctx.send(reply).await?;
    let leaderboard = ctx.data().db.get_leaderboard().await?;

    let chosen_players = leaderboard
        .choose_multiple(&mut rand::thread_rng(), 4)
        .cloned()
        .collect::<Vec<_>>();

    let mut named_players = HashMap::new();

    for player in chosen_players.iter() {
        let name = get_discord_name(ctx, &player.0).await;
        named_players.insert(player.0.clone(), name);
    }

    let components = vec![CreateActionRow::Buttons(vec![
        new_vote_for_user_button(
            &chosen_players[0].0,
            named_players.get(&chosen_players[0].0).unwrap(),
        ),
        new_vote_for_user_button(
            &chosen_players[1].0,
            named_players.get(&chosen_players[1].0).unwrap(),
        ),
        new_vote_for_user_button(
            &chosen_players[2].0,
            named_players.get(&chosen_players[2].0).unwrap(),
        ),
    ])];
    let reply = {
        CreateMessage::default()
            .content("Time for some wealth distrubution! Which one of these players could spare a couple of bucks?".to_string())
            .components(components.clone())
    };

    let id = ctx.channel_id().send_message(ctx, reply).await?;
    let mut votes = HashMap::new();
    let mut already_voted = HashSet::new();
    while let Some(mci) = ComponentInteractionCollector::new(ctx)
        .channel_id(ctx.channel_id())
        .message_id(id.id)
        .timeout(std::time::Duration::from_secs(
            5, // (now + time_to_play - 1) - SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        ))
        .await
    {
        let voter_id = mci.user.id;
        let choice = mci.data.custom_id.clone();
        dbg!(voter_id.to_string(), &choice);
        if already_voted.contains(&voter_id) {
            mci.create_response(ctx, CreateInteractionResponse::Acknowledge)
                .await?;
            continue;
        }
        dbg!(voter_id.to_string(), &choice);

        if voter_id.to_string() == choice {
            mci.create_response(
                ctx,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("You can't vote for yourself!".to_string())
                        .ephemeral(true),
                ),
            )
            .await?;
            continue;
        }

        already_voted.insert(voter_id);
        if let Some(x) = votes.get_mut(&choice) {
            *x += 1;
        } else {
            votes.insert(choice, 1);
        }

        mci.create_response(ctx, CreateInteractionResponse::Acknowledge)
            .await?;
    }
    // get the highest voted player
    let (player, _) = votes.iter().max_by_key(|x| x.1).unwrap();
    let message = {
        CreateReply::default()
            .content(
                format!(
                    "Awoo, we just robbed X from <@{}>! I hope you are proud {}. You each get {}:dollar:",
                    player,
                    "robbers".to_owned(),
                    10
                )
                .to_string(),
            )
            .allowed_mentions(CreateAllowedMentions::new().empty_users())
    };
    ctx.send(message).await?;
    Ok(())
}

fn new_vote_for_user_button(user: &String, name: &String) -> CreateButton {
    CreateButton::new(user)
        .label(name.to_string())
        .style(poise::serenity_prelude::ButtonStyle::Primary)
}

async fn get_discord_name(ctx: Context<'_>, user: &str) -> String {
    let user = poise::serenity_prelude::UserId::new(user.parse().unwrap())
        .to_user(ctx)
        .await
        .unwrap();
    user.nick_in(ctx, ctx.guild_id().unwrap())
        .await
        .unwrap_or(user.name)
}
