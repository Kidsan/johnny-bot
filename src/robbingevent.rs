use crate::{database::BalanceDatabase, Context, Error};
use rand::{seq::SliceRandom, Rng};
use serenity::all::{
    ComponentInteractionCollector, CreateActionRow, CreateAllowedMentions, CreateButton,
    CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage, EditMessage,
};
use std::{
    collections::{HashMap, HashSet},
    time::{SystemTime, UNIX_EPOCH},
};
///
/// Start a robbing event
///
/// Enter `/robbingevent` to start a robbing event. This will randomly select 4 players from the leaderboard and ask the chat to vote on who to rob from.
/// Requires that there be 4 players on the leaderboard. Will fail if one of the chosen players has
/// 0 bucks
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
    if leaderboard.len() < 4 {
        let reply = {
            poise::CreateReply::default()
                .content("Not enough players to rob from.")
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Ok(());
    }

    let chosen_players = leaderboard
        .choose_multiple(&mut rand::thread_rng(), 4)
        .cloned()
        .collect::<Vec<_>>();

    let mut named_players = HashMap::new();

    for player in chosen_players.iter() {
        if player.1 == 0 {
            let reply = {
                poise::CreateReply::default()
                    .content(
                        "One of the chosen players has no money, so we're skipping this round.",
                    )
                    .ephemeral(true)
            };
            ctx.send(reply).await?;
            return Ok(());
        }
        let name = get_discord_name(ctx, &player.0).await;
        named_players.insert(player.0.clone(), name);
        ctx.data()
            .locked_balances
            .lock()
            .unwrap()
            .insert(player.0.clone());
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
        new_vote_for_user_button(
            &chosen_players[3].0,
            named_players.get(&chosen_players[3].0).unwrap(),
        ),
    ])];
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let time_to_play = ctx.data().game_length;

    let reply = {
        CreateMessage::default()
            .content(format!(
                    "> ### :coin: Time for some wealth distrubution!\n> Which one of these players could spare a couple of bucks?\n > **Voting Ends: **<t:{}:R>", now+time_to_play))
            .components(components.clone())
    };

    let mut id = ctx.channel_id().send_message(ctx, reply).await?;
    let mut votes: HashMap<String, Vec<String>> = HashMap::new();
    let mut already_voted: HashSet<String> = HashSet::new();

    for player in chosen_players.iter() {
        votes.insert(player.0.clone(), vec![]);
    }

    while let Some(mci) = ComponentInteractionCollector::new(ctx)
        .channel_id(ctx.channel_id())
        .message_id(id.id)
        .timeout(std::time::Duration::from_secs(
            (now + time_to_play - 1) - SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        ))
        .await
    {
        let voter_id = mci.user.id;
        let choice = mci.data.custom_id.clone();
        if already_voted.contains(&voter_id.to_string()) {
            mci.create_response(
                ctx,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("You have already voted".to_string())
                        .ephemeral(true),
                ),
            )
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

        already_voted.insert(voter_id.to_string());
        if let Some(x) = votes.get_mut(&choice) {
            x.push(voter_id.to_string());
        } else {
            let v = vec![voter_id.to_string()];
            votes.insert(choice, v);
        }

        mci.create_response(ctx, CreateInteractionResponse::Acknowledge)
            .await?;
    }

    let components = vec![CreateActionRow::Buttons(vec![
        new_vote_for_user_button(
            &chosen_players[0].0,
            named_players.get(&chosen_players[0].0).unwrap(),
        )
        .disabled(true),
        new_vote_for_user_button(
            &chosen_players[1].0,
            named_players.get(&chosen_players[1].0).unwrap(),
        )
        .disabled(true),
        new_vote_for_user_button(
            &chosen_players[2].0,
            named_players.get(&chosen_players[2].0).unwrap(),
        )
        .disabled(true),
        new_vote_for_user_button(
            &chosen_players[3].0,
            named_players.get(&chosen_players[3].0).unwrap(),
        )
        .disabled(true),
    ])];

    let reply = {
        EditMessage::default()
            .content("> ### :coin: Time for some wealth distrubution!\n> Which one of these players could spare a couple of bucks?\n > **Voting Has Ended!**".to_string())
            .components(components.clone())
    };

    id.edit(ctx, reply).await?;

    // get the highest voted player
    let (player, _) = votes.iter().max_by_key(|x| x.1.len()).unwrap();
    let robbers = votes.get(player).unwrap();
    let robber_list = robbers
        .iter()
        .map(|x| format!("<@{}>", x))
        .collect::<Vec<String>>()
        .join(", ");

    let percentage_to_steal = ctx.data().rng.lock().unwrap().gen_range(5..=25);

    let balance = ctx.data().db.get_balance(player.to_string()).await?;
    let stolen = balance * percentage_to_steal / 100;

    if robbers.is_empty() {
        let message = {
            CreateMessage::default()
                .content("Wow! Noone wants to rob anyone. Either the chat is dead or this is... kind of wholesome.")
                .allowed_mentions(CreateAllowedMentions::new().empty_users())
                .reference_message(&id)
        };
        for user in chosen_players.iter() {
            ctx.data().locked_balances.lock().unwrap().remove(&user.0);
        }
        ctx.channel_id().send_message(ctx, message).await?;
        return Ok(());
    }

    let each = stolen / robbers.len() as i32;

    ctx.data().db.award_balances(robbers.to_vec(), each).await?;
    ctx.data()
        .db
        .subtract_balances(vec![player.to_string()], stolen)
        .await?;

    let text = match robbers.len() == 1 {
        false => format!("Awoo, we just robbed {}:dollar: from <@{}>! I hope you are proud {}. You each get {}:dollar:!", stolen,
                    player,
                    robber_list,
                    each
                )
                .to_string(),
        true => format!(
                    "Awoo, we just robbed {}:dollar: from <@{}>! I hope you are proud {}. You get {}:dollar:!",
                    stolen,
                    player,
                    robber_list,
                    each
                )
                .to_string(),
    };

    let message = {
        CreateMessage::default()
            .content(text)
            .allowed_mentions(CreateAllowedMentions::new().empty_users())
            .reference_message(&id)
    };
    for user in chosen_players.iter() {
        ctx.data().locked_balances.lock().unwrap().remove(&user.0);
    }
    ctx.channel_id().send_message(ctx, message).await?;
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
