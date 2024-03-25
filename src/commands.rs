use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
    vec,
};

use rand::seq::SliceRandom;

use crate::{Context, Error};
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
pub async fn start_gamble(
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
    let time_to_play = 30;
    let mut players = vec![game_starter];
    let mut pot = amount;
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

    while let Some(mci) = serenity::ComponentInteractionCollector::new(ctx)
        // .author_id(ctx.author().id) this filters to interactions just by the user
        .channel_id(ctx.channel_id())
        .timeout(std::time::Duration::from_secs(
            (now + time_to_play - 1) - SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        ))
        .filter(move |mci| mci.data.custom_id == "Bet" && mci.message.id == id)
        .await
    {
        dbg!(&mci);
        let player = mci.user.id.to_string();
        if players.contains(&player) {
            mci.create_response(ctx, serenity::CreateInteractionResponse::Acknowledge)
                .await?;
            continue;
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
            {
                let mut balances = ctx.data().balances.lock().unwrap();
                let player_balance = balances.entry(player).or_default();
                *player_balance -= amount;
            }
            players.push(mci.user.to_string());
            pot += amount;

            let mut msg = mci.message.clone();

            msg.edit(
                ctx,
                serenity::EditMessage::new().components(vec![serenity::CreateActionRow::Buttons(
                    vec![
                        new_bet_button(amount),
                        new_player_count_button(players.len() as i32),
                        new_pot_counter_button(pot),
                    ],
                )]),
            )
            .await?;

            mci.create_response(ctx, serenity::CreateInteractionResponse::Acknowledge)
                .await?;
        }
    }
    let winner = players.choose(&mut rand::thread_rng()).unwrap();
    {
        let mut balances = ctx.data().balances.lock().unwrap();
        let user_balance = balances.entry(winner.clone()).or_insert(50);
        *user_balance += pot;
    }
    a.edit(
        ctx,
        CreateReply::default()
            .content(format!(
                "Game is over, winner is: {}, they won: {} J-Bucks!",
                ctx.author(),
                pot
            ))
            .components(vec![serenity::CreateActionRow::Buttons(vec![
                new_bet_button(amount).disabled(true),
                new_player_count_button(players.len() as i32),
                new_pot_counter_button(pot),
            ])]),
    )
    .await?;
    Ok(())
}

/// View Leaderboard
///
/// Enter `~leaderboard` to view
/// ````
/// /leaderboard
/// ```
#[poise::command(prefix_command, slash_command)]
pub async fn leaderboard(ctx: Context<'_>) -> Result<(), Error> {
    let mut response = String::new();
    for i in 0..9 {
        response += format!("{}. {} with {} J-Bucks!\n", i + 1, ctx.author(), 50).as_str();
    }
    ctx.say(response).await?;
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
