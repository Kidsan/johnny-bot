use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
    vec,
};

use rand::seq::SliceRandom;

use crate::{Context, Error, Game};
use poise::{
    serenity_prelude::{self as serenity, CreateInteractionResponseMessage},
    CreateReply,
};

#[poise::command(prefix_command, track_edits, slash_command)]
pub async fn register(ctx: Context<'_>) -> Result<(), Error> {
    poise::builtins::register_application_commands_buttons(ctx).await?;
    Ok(())
}

/// Show this help menu
#[poise::command(prefix_command, track_edits, slash_command)]
pub async fn help(
    ctx: Context<'_>,
    #[description = "Specific command to show help about"]
    #[autocomplete = "poise::builtins::autocomplete_command"]
    command: Option<String>,
) -> Result<(), Error> {
    poise::builtins::help(
        ctx,
        command.as_deref(),
        poise::builtins::HelpConfiguration {
            extra_text_at_bottom: "This is an example bot made to showcase features of my custom Discord bot framework",
            ..Default::default()
        },
    )
    .await?;
    Ok(())
}

/// Check your balance
///
/// Enter `~checkbucks` to check
/// ````
/// /checkbucks
/// ```
#[poise::command(prefix_command, slash_command)]
pub async fn checkbucks(ctx: Context<'_>) -> Result<(), Error> {
    let user_balance;
    {
        let mut balances = ctx.data().balances.lock().unwrap();
        user_balance = balances
            .entry(ctx.author().id.to_string())
            .or_insert(50)
            .to_owned()
    }
    let reply = {
        CreateReply::default()
            .content(format!("{} has {} J-Bucks!", ctx.author(), user_balance,))
            .ephemeral(true)
    };
    ctx.send(reply).await?;
    Ok(())
}

fn new_bet_button(amount: i32) -> serenity::CreateButton {
    serenity::CreateButton::new("Bet")
        .label(format!("Bet {} J-Bucks", amount))
        .style(poise::serenity_prelude::ButtonStyle::Primary)
}
fn new_player_count_button(amount: i32) -> serenity::CreateButton {
    serenity::CreateButton::new("Players")
        .label(format!("Players: {} ", amount))
        .disabled(true)
        .style(poise::serenity_prelude::ButtonStyle::Secondary)
}
fn new_pot_counter_button(amount: i32) -> serenity::CreateButton {
    serenity::CreateButton::new("Pot")
        .label(format!("Total Pot: {} ", amount))
        .disabled(true)
        .style(poise::serenity_prelude::ButtonStyle::Success)
}

fn user_can_play(user_balance: i32, amount: i32) -> bool {
    user_balance >= amount
}

/// Start a gamble
///
/// Enter `~startGamble` to play
/// ````
/// /startGamble
/// ```
#[poise::command(prefix_command, track_edits, slash_command)]
pub async fn gamble(
    ctx: Context<'_>,
    #[description = "amount to play"] amount: i32,
) -> Result<(), Error> {
    let game_starter = ctx.author().id.to_string();
    let mut start_game: bool = true;
    {
        let mut balances = ctx.data().balances.lock().unwrap();
        if !balances.contains_key(&ctx.author().id.to_string()) {
            balances.insert(game_starter.clone(), 50);
        }

        if user_can_play(*balances.get(&game_starter).unwrap(), amount) {
            let user_balance = balances.entry(game_starter.clone()).or_default();
            *user_balance -= amount;
        } else {
            start_game = false;
        }
    }

    if !start_game {
        let reply = {
            CreateReply::default()
                .content("You can't afford to do that!")
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("You can't afford to do that".into());
    }

    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let time_to_play = 10;
    let players = vec![game_starter.clone()];
    let pot = amount;
    let components = vec![serenity::CreateActionRow::Buttons(vec![
        new_bet_button(amount),
        new_player_count_button(players.len() as i32),
        new_pot_counter_button(pot),
    ])];
    let reply = {
        CreateReply::default()
            .content(format!(
                "{} has started a game, place your bets!\n Betting deadline <t:{}:R>",
                ctx.author(),
                now + time_to_play
            ))
            .components(components.clone())
    };

    let a = ctx.send(reply).await?;
    let id = a.message().await?.id;
    {
        let mut games = ctx.data().games.lock().unwrap();
        games.insert(
            id.to_string(),
            Game {
                id: id.to_string(),
                players: players.clone(),
                amount,
                pot: amount,
            },
        );
    }

    while let Some(mci) = serenity::ComponentInteractionCollector::new(ctx)
        // .author_id(ctx.author().id) this filters to interactions just by the user
        .channel_id(ctx.channel_id())
        .message_id(id)
        .timeout(std::time::Duration::from_secs(
            (now + time_to_play - 1) - SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        ))
        .filter(move |mci| mci.data.custom_id == "Bet")
        .await
    {
        dbg!(&mci);
        let player = mci.user.id.to_string();
        {
            if ctx
                .data()
                .games
                .lock()
                .unwrap()
                .get(&id.to_string())
                .unwrap()
                .players
                .contains(&player)
            {
                mci.create_response(ctx, serenity::CreateInteractionResponse::Acknowledge)
                    .await?;
                continue;
            }
        }

        let player_balance;
        {
            let mut balances = ctx.data().balances.lock().unwrap();
            player_balance = *balances.entry(player.clone()).or_insert(50);
        }
        if player_balance < amount {
            mci.create_response(
                ctx,
                serenity::CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("You can't afford to do that!")
                        .ephemeral(true),
                ),
            )
            .await?;
        } else {
            let button2;
            let button3;
            {
                let mut balances = ctx.data().balances.lock().unwrap();
                let player_balance = balances.entry(player).or_insert(50);
                *player_balance -= amount;

                let mut games = ctx.data().games.lock().unwrap();
                let game = games.get_mut(&id.to_string()).unwrap();
                game.players.push(mci.user.id.to_string());
                game.pot += amount;
                button2 = new_player_count_button(game.players.len() as i32);
                button3 = new_pot_counter_button(game.pot);
            }

            let mut msg = mci.message.clone();

            msg.edit(
                ctx,
                serenity::EditMessage::new().components(vec![serenity::CreateActionRow::Buttons(
                    vec![new_bet_button(amount), button2, button3],
                )]),
            )
            .await?;
        }
        mci.create_response(ctx, serenity::CreateInteractionResponse::Acknowledge)
            .await?;
    }
    let winner;
    {
        let mut games = ctx.data().games.lock().unwrap();
        let game = games.get_mut(&id.to_string()).unwrap();
        winner = game
            .players
            .choose(&mut rand::thread_rng())
            .unwrap()
            .clone();
        {
            let mut balances = ctx.data().balances.lock().unwrap();
            let user_balance = balances.get_mut(&winner).unwrap();
            *user_balance += game.pot;
        }
    }
    let button2;
    let button3;
    let prize;
    {
        let mut games = ctx.data().games.lock().unwrap();
        let game = games.get_mut(&id.to_string()).unwrap();
        button2 = new_player_count_button(game.players.len() as i32);
        button3 = new_pot_counter_button(game.pot);
        prize = game.pot;
    }
    let winner_id = winner.parse().unwrap();
    a.edit(
        ctx,
        CreateReply::default()
            .content(format!(
                "Game is over, winner is: {}, they won: {} J-Bucks!",
                // winner,
                serenity::UserId::new(winner_id).to_user(ctx).await?,
                prize
            ))
            .components(vec![serenity::CreateActionRow::Buttons(vec![
                new_bet_button(amount).disabled(true),
                button2,
                button3,
            ])]),
    )
    .await?;
    // see if we can find a nice way to tell the user their balance after they win
    Ok(())
}

pub async fn get_discord_users(
    ctx: Context<'_>,
    user_ids: Vec<String>,
) -> Result<HashMap<String, poise::serenity_prelude::User>, Error> {
    let mut users = HashMap::new();
    for user_id in user_ids {
        let user = serenity::UserId::new(user_id.parse().unwrap())
            .to_user(ctx)
            .await?;
        users.insert(user_id, user);
    }
    Ok(users)
}

/// View Leaderboard
///
/// Enter `~leaderboard` to view
/// ````
/// /leaderboard
/// ```
#[poise::command(prefix_command, slash_command)]
pub async fn leaderboard(ctx: Context<'_>) -> Result<(), Error> {
    let mut top_players: Vec<(&String, &i32)> = vec![];
    let ids;
    let top;
    {
        let balances = ctx.data().balances.lock().unwrap().clone();
        let mut a: Vec<(&String, &i32)> = balances.iter().collect();
        a.sort_by(|a, b| b.1.cmp(a.1));
        for f in a.iter().take(10) {
            let a = *f;
            top_players.push((a.0, a.1))
        }
        ids = top_players
            .clone()
            .into_iter()
            .map(|(k, _v)| k)
            .map(|x| x.to_string())
            .collect::<Vec<String>>();
        let players_resolved = get_discord_users(ctx, ids).await?;

        top = top_players
            .iter()
            .map(|(k, v)| (players_resolved.get(*k).unwrap(), v))
            .enumerate()
            .map(|(i, (k, v))| format!("{}: {} with {} J-Bucks!", i + 1, k.name, v))
            .collect::<Vec<_>>()
            .join("\n");
    }
    if top.is_empty() {
        ctx.say("Nobody has any J-Bucks yet!").await?;
        return Ok(());
    }

    ctx.say(top).await?;
    Ok(())
}

/// Vote for something
///
/// Enter `~vote pumpkin` to vote for pumpkins
#[poise::command(prefix_command, slash_command)]
pub async fn vote(
    ctx: Context<'_>,
    #[description = "What to vote for"] choice: String,
) -> Result<(), Error> {
    // Lock the Mutex in a block {} so the Mutex isn't locked across an await point
    let num_votes: i32 = {
        let mut hash_map = HashMap::new();
        let num_votes = hash_map.entry(choice.clone()).or_default();
        *num_votes += 1;
        *num_votes
    };

    let response = format!("Successfully voted for {choice}. {choice} now has {num_votes} votes!");
    ctx.say(response).await?;
    Ok(())
}

/// Retrieve number of votes
///
/// Retrieve the number of votes either in general, or for a specific choice:
/// ```
/// ~getvotes
/// ~getvotes pumpkin
/// ```
#[poise::command(prefix_command, track_edits, aliases("votes"), slash_command)]
pub async fn getvotes(
    ctx: Context<'_>,
    #[description = "Choice to retrieve votes for"] choice: Option<String>,
) -> Result<(), Error> {
    if let Some(choice) = choice {
        let data: HashMap<String, i32> = HashMap::new();
        let num_votes = *data.get(&choice).unwrap_or(&0);
        let response = match num_votes {
            0 => format!("Nobody has voted for {} yet", choice),
            _ => format!("{} people have voted for {}", num_votes, choice),
        };
        ctx.say(response).await?;
    } else {
        let mut response = String::new();
        let data: HashMap<String, i32> = HashMap::new();
        for (choice, num_votes) in data.iter() {
            response += &format!("{}: {} votes", choice, num_votes);
        }

        if response.is_empty() {
            response += "Nobody has voted for anything yet :(";
        }

        ctx.say(response).await?;
    };

    Ok(())
}
