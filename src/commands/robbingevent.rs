use crate::{
    database::BalanceDatabase, database::RobberyDatabase, database::RoleDatabase, Context, Error,
};
use chrono::{Datelike, NaiveDate, NaiveTime};
use poise::serenity_prelude;
use rand::{seq::SliceRandom, Rng};
use serenity::{
    all::{
        ActivityData, ComponentInteractionCollector, CreateActionRow, CreateAllowedMentions,
        CreateButton, CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage,
        EditMessage,
    },
    futures::future::join_all,
};
use std::{
    collections::{HashMap, HashSet},
    time::{SystemTime, UNIX_EPOCH},
};

async fn no_locked_balances(ctx: Context<'_>) -> Result<bool, Error> {
    if ctx.data().locked_balances.lock().unwrap().is_empty() {
        Ok(true)
    } else {
        let reply = {
            poise::CreateReply::default()
                .content("There is already a robbing event in progress!")
                .ephemeral(true)
        };
        let _ = ctx.send(reply).await;
        Ok(false)
    }
}

async fn enough_players(ctx: Context<'_>) -> Result<bool, Error> {
    let leaderboard = ctx.data().db.get_leaderboard().await?;
    if leaderboard.len() < 4 {
        let reply = {
            poise::CreateReply::default()
                .content("Not enough players to rob from.")
                .ephemeral(true)
        };
        let _ = ctx.send(reply).await;
        Ok(false)
    } else {
        Ok(true)
    }
}

///
/// Start a robbing event
///
/// Enter `/robbingevent` to start a robbing event. This will randomly select 4 players from the leaderboard and ask the chat to vote on who to rob from.
/// Requires that there be 4 players on the leaderboard. Will fail if one of the chosen players has
/// 0 bucks
/// ```
/// /coingamble 10
/// ```
#[poise::command(
    slash_command,
    category = "Admin",
    default_member_permissions = "ADMINISTRATOR",
    hide_in_help,
    check = "no_locked_balances",
    check = "enough_players"
)]
pub async fn robbingevent(ctx: Context<'_>) -> Result<(), Error> {
    let reply = {
        poise::CreateReply::default()
            .content("Success!")
            .ephemeral(true)
    };
    ctx.send(reply).await?;
    wrapped_robbing_event(ctx, None).await?;
    Ok(())
}

///
/// pay 10 J-Bucks to start a robbing event
///
/// Enter `/buy robbery` to start a robbing event. Event costs 10 JBucks This will randomly select 4 players from the leaderboard and ask the chat to vote on who to rob from.
/// Requires that there be 4 players on the leaderboard. Will fail if one of the chosen players has
/// 0 bucks
/// ```
/// /buy robbery
/// ```
#[poise::command(
    slash_command,
    rename = "robbery",
    check = "no_locked_balances",
    check = "enough_players"
)]
pub async fn buyrobbery(ctx: Context<'_>) -> Result<(), Error> {
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
    match robbery_cooldown(ctx).await {
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
    ctx.data()
        .active_checks
        .lock()
        .unwrap()
        .remove(&(ctx.author().id.get()));
    let user_balance = ctx.data().db.get_balance(ctx.author().id.get()).await?;
    if user_balance < 10 {
        let reply = {
            poise::CreateReply::default()
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
        .subtract_balances(vec![ctx.author().id.get()], 10)
        .await?;

    let reply = {
        poise::CreateReply::default()
            .content("Success!")
            .ephemeral(true)
    };
    ctx.send(reply).await?;
    wrapped_robbing_event(ctx, Some(ctx.author().clone())).await?;
    ctx.data().db.bought_robbery(ctx.author().id.get()).await?;
    Ok(())
}

pub async fn wrapped_robbing_event(
    ctx: Context<'_>,
    user: Option<serenity_prelude::User>,
) -> Result<(), Error> {
    if !ctx.data().locked_balances.lock().unwrap().is_empty() {
        tracing::info!("locked balances not empty, aborting robbing event");
        return Ok(());
    }
    let leaderboard = ctx.data().db.get_leaderboard().await?;
    let chosen_players = leaderboard
        .choose_multiple(&mut rand::thread_rng(), 4)
        .cloned()
        .collect::<Vec<_>>();

    let mut named_players = HashMap::new();
    let mut abort = false;

    {
        let mut locked = ctx.data().locked_balances.lock().unwrap();
        for player in chosen_players.iter() {
            if player.1 == 0 {
                // clear locked balances
                ctx.data().locked_balances.lock().unwrap().clear();
                abort = true;
            }
            locked.insert(player.0);
        }
    }

    if abort {
        let reply = {
            poise::CreateReply::default()
                .content("One of the chosen players has no money, so we're skipping this round.")
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Ok(());
    }
    let players = { ctx.data().locked_balances.lock().unwrap().clone() };
    for player in players {
        let name = get_discord_name(ctx, player).await;
        named_players.insert(player, name);
    }

    let components = vec![CreateActionRow::Buttons(vec![
        new_vote_for_user_button(
            chosen_players[0].0,
            named_players.get(&chosen_players[0].0).unwrap(),
        ),
        new_vote_for_user_button(
            chosen_players[1].0,
            named_players.get(&chosen_players[1].0).unwrap(),
        ),
        new_vote_for_user_button(
            chosen_players[2].0,
            named_players.get(&chosen_players[2].0).unwrap(),
        ),
        new_vote_for_user_button(
            chosen_players[3].0,
            named_players.get(&chosen_players[3].0).unwrap(),
        ),
    ])];
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let time_to_play = { ctx.data().config.read().unwrap().robbery_length_seconds };

    ctx.serenity_context()
        .shard
        .set_activity(Some(ActivityData::custom("redistributing wealth")));

    let msg = match user {
        Some(u) => format!("{} has started a wealth redistribution!", u),
        None => "Time for some wealth redistribution!".to_string(),
    };
    let reply = {
        CreateMessage::default()
            .content(format!(
                    "> ### <:jbuck:1228663982462865450> {}\n> Which one of these players could spare a couple of bucks?\n > **Voting Ends: **<t:{}:R>", msg, now+time_to_play as u64))
            .components(components.clone())
    };

    let mut id = ctx.channel_id().send_message(ctx, reply).await?;
    let mut votes: HashMap<String, Vec<String>> = HashMap::new();
    let mut already_voted: HashSet<String> = HashSet::new();

    for player in chosen_players.iter() {
        votes.insert(player.0.to_string(), vec![]);
    }

    while let Some(mci) = ComponentInteractionCollector::new(ctx)
        .channel_id(ctx.channel_id())
        .message_id(id.id)
        .timeout(std::time::Duration::from_secs(
            (now + time_to_play as u64 - 1)
                - SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
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
            votes.insert(choice.clone(), v);
        }

        // ensures the voter has a balance
        let _ = ctx.data().db.get_balance(voter_id.get()).await?;

        mci.create_response(
            ctx,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(format!("You have voted for <@{}>", &choice))
                    .allowed_mentions(CreateAllowedMentions::new().empty_users())
                    .ephemeral(true),
            ),
        )
        .await?;
    }

    let components = vec![CreateActionRow::Buttons(vec![
        new_vote_for_user_button(
            chosen_players[0].0,
            named_players.get(&chosen_players[0].0).unwrap(),
        )
        .disabled(true),
        new_vote_for_user_button(
            chosen_players[1].0,
            named_players.get(&chosen_players[1].0).unwrap(),
        )
        .disabled(true),
        new_vote_for_user_button(
            chosen_players[2].0,
            named_players.get(&chosen_players[2].0).unwrap(),
        )
        .disabled(true),
        new_vote_for_user_button(
            chosen_players[3].0,
            named_players.get(&chosen_players[3].0).unwrap(),
        )
        .disabled(true),
    ])];

    let reply = {
        EditMessage::default()
            .content(format!("> ### <:jbuck:1228663982462865450> {}\n> Which one of these players could spare a couple of bucks?\n > **Voting Has Ended!**", msg))
            .components(components.clone())
    };

    id.edit(ctx, reply).await?;

    let mut crowns_vote = None;

    if let Some(user) = ctx
        .data()
        .db
        .get_unique_role_holder(ctx.data().crown_role_id)
        .await?
    {
        let crown_holder_id = user.user_id;
        for (player, votes) in votes.iter() {
            if votes.contains(&crown_holder_id.to_string()) {
                crowns_vote = Some(player.clone());
            }
        }
    }

    let (player, robbers) = if let Some(ref u) = crowns_vote {
        (u.clone(), votes.get(u).unwrap().clone())
    } else {
        match votes
            .iter()
            .filter(|x| !x.1.is_empty())
            .collect::<Vec<_>>()
            .choose(&mut rand::thread_rng())
        {
            Some(x) => (x.0.clone(), x.1.clone()),
            None => ("".to_string(), vec![]),
        }
    };

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
        ctx.serenity_context().shard.set_activity(None);
        return Ok(());
    }
    let robber_list = robbers
        .iter()
        .map(|x| format!("<@{}>", x))
        .collect::<Vec<String>>()
        .join(", ");

    let percentage_to_steal = ctx.data().rng.lock().unwrap().gen_range(5..=25);

    let balance = ctx.data().db.get_balance(player.parse().unwrap()).await?;
    let stolen = balance * percentage_to_steal / 100;

    let each = stolen / robbers.len() as i32;

    let victim_name = named_players.get(&player.parse().unwrap()).unwrap().clone();

    if each == 0 {
        let message = {
            CreateMessage::default()
                .content(format!("> ### <:jbuck:1228663982462865450> Awoo, we just tried to rob {} but they are too poor!\n> I hope you are proud {}.", victim_name, robber_list).to_string())
                .allowed_mentions(CreateAllowedMentions::new().empty_users())
                .reference_message(&id)
        };
        for user in chosen_players.iter() {
            ctx.data().locked_balances.lock().unwrap().remove(&user.0);
        }
        ctx.channel_id().send_message(ctx, message).await?;
        ctx.serenity_context().shard.set_activity(None);
        return Ok(());
    }

    ctx.data()
        .db
        .award_balances(robbers.iter().map(|z| z.parse().unwrap()).collect(), each)
        .await?;
    ctx.data()
        .db
        .subtract_balances(vec![player.parse().unwrap()], stolen)
        .await?;

    let mut text = format!("> ### <:jbuck:1228663982462865450> {}\n> I hope you are proud {}.\n> **You {}get {} <:jbuck:1228663982462865450>!**",
        if let Some(_u) = crowns_vote {
            format!("The crown chose {}, we just robbed {} <:jbuck:1228663982462865450> from them!", victim_name,stolen)
        } else {
            format!("Awoo, we just robbed {} <:jbuck:1228663982462865450> from {}!", stolen, victim_name)
        },
        robber_list,
        if robbers.len() == 1 { "" } else { "each " },
        each);

    let names: Vec<_> = join_all(
        votes
            .iter()
            .filter(|x| x.0 != &player && !x.1.is_empty())
            .map(|a| async {
                let name = get_discord_name(ctx, a.0.parse().unwrap()).await;
                (a.0.to_owned(), name)
            }),
    )
    .await;

    let names_map: HashMap<String, String> = HashMap::from_iter(names.into_iter());

    let formatted_other_votes = votes
        .iter()
        .filter(|x| x.0 != &player && !x.1.is_empty())
        .map(|a| {
            let person = names_map.get(a.0).unwrap();
            let users =
                a.1.iter()
                    .map(|x| format!("<@{}>", x))
                    .collect::<Vec<String>>()
                    .join(", ");
            format!("> **{}**: {}", person, users)
        })
        .collect::<Vec<String>>()
        .join("\n");

    if !formatted_other_votes.is_empty() {
        text = format!("{}\n> \n> Other votes:\n{}", text, formatted_other_votes);
    }

    let message = {
        CreateMessage::default()
            .content(text)
            .allowed_mentions(CreateAllowedMentions::new().empty_users())
            .reference_message(&id)
    };
    ctx.data().locked_balances.lock().unwrap().clear();
    ctx.channel_id().send_message(ctx, message).await?;
    ctx.serenity_context().shard.set_activity(None);
    Ok(())
}

fn new_vote_for_user_button(user: u64, name: &String) -> CreateButton {
    CreateButton::new(user.to_string())
        .label(name.to_string())
        .style(poise::serenity_prelude::ButtonStyle::Primary)
}

pub async fn get_discord_name(ctx: Context<'_>, user: u64) -> String {
    let user = poise::serenity_prelude::UserId::new(user)
        .to_user(ctx)
        .await
        .unwrap();
    user.nick_in(ctx, ctx.guild_id().unwrap())
        .await
        .unwrap_or(user.name)
}

// robbery is only possible once per week
async fn robbery_cooldown(ctx: Context<'_>) -> Result<(), Error> {
    let week_number = chrono::Utc::now().date_naive().iso_week().week();

    let (start, end) = week_bounds(week_number);

    let last_robbery = ctx
        .data()
        .db
        .get_last_bought_robbery(ctx.author().id.get())
        .await?;

    if last_robbery.is_none() {
        return Ok(());
    }

    if last_robbery.unwrap().naive_utc() > start.into() {
        let ts = end
            .and_time(NaiveTime::from_hms_opt(23, 59, 59).unwrap())
            .and_utc()
            .timestamp();

        let reply = {
            poise::CreateReply::default()
                .content(format!(
                    "You can only do this once per week! Try again <t:{}:R>.",
                    ts
                ))
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("You can only do this once per week.".to_string().into());
    }
    Ok(())
}

pub fn week_bounds(week: u32) -> (NaiveDate, NaiveDate) {
    let current_year = chrono::offset::Local::now().year();
    (
        NaiveDate::from_isoywd_opt(current_year, week, chrono::Weekday::Mon).unwrap(),
        NaiveDate::from_isoywd_opt(current_year, week, chrono::Weekday::Sun).unwrap(),
    )
}
