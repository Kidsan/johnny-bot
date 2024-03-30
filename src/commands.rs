use std::{
    collections::HashMap,
    time::{self, SystemTime, UNIX_EPOCH},
    vec,
};

use crate::{game::Game, Context, Error};
use poise::{
    serenity_prelude::{self as serenity, CreateInteractionResponseMessage, User, UserId},
    CreateReply,
};
use rusqlite::params;

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

pub async fn get_user_balance(
    user_id: String,
    conn: &tokio_rusqlite::Connection,
) -> Result<i32, Error> {
    let user = user_id.clone();
    let balance = conn
        .call(move |conn| {
            let mut stmt = conn.prepare_cached("SELECT balance FROM balances WHERE id = (?1)")?;
            Ok(stmt.query_row(params![user], |row| {
                let balance: i32 = row.get(0)?;

                Ok(balance)
            }))
        })
        .await?;
    let result = match balance {
        Ok(user_balance) => user_balance,
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            let user = user_id;
            let _ = conn
                .call(move |conn| {
                    let mut stmt =
                        conn.prepare_cached("INSERT INTO balances (id, balance) VALUES (?1, ?2)")?;
                    Ok(stmt.query_row(params![user, 50], |row| {
                        let balance: i32 = row.get(0)?;

                        Ok(balance)
                    }))
                })
                .await?;
            50
        }
        Err(e) => return Err(e.into()),
    };
    Ok(result)
}
pub async fn set_user_balance(
    user_id: String,
    amount: i32,
    conn: &tokio_rusqlite::Connection,
) -> Result<(), Error> {
    let user = user_id.clone();
    let _ = conn
        .call(move |conn| {
            let mut stmt =
                conn.prepare_cached("UPDATE balances SET balance = (?1) WHERE id = (?2)")?;
            Ok(stmt.execute(params![amount, user]))
        })
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
    let user_id = ctx.author().id.to_string();
    let response = get_user_balance(user_id, &ctx.data().db).await?;
    let reply = {
        CreateReply::default()
            .content(format!("{} has {} J-Bucks!", ctx.author(), response,))
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
/// Enter `~gamble` to play
/// ````
/// /gamble
/// ```
#[poise::command(prefix_command, track_edits, slash_command)]
pub async fn gamble(
    ctx: Context<'_>,
    #[description = "amount to play"] amount: i32,
) -> Result<(), Error> {
    let game_starter = ctx.author().id.to_string();
    let db = &ctx.data().db;
    let mut start_game: bool = true;
    let user_balance;
    {
        user_balance = get_user_balance(game_starter.clone(), db).await?;

        if user_can_play(user_balance, amount) {
            set_user_balance(game_starter.clone(), user_balance - amount, db).await?;
        } else {
            start_game = false;
        }
    }

    if !start_game {
        let reply = {
            CreateReply::default()
                .content(format!(
                    "You can't afford to do that!\nYour balance is {} J-Bucks.",
                    user_balance
                ))
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("You can't afford to do that".into());
    }

    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let time_to_play = 10;
    let players = [game_starter.clone()];
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
            Game::new(
                id.to_string(),
                amount,
                ctx.author().id.to_string(),
                time::Instant::now(),
            ),
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

        let player_balance = get_user_balance(player.clone(), db).await?;

        if user_can_play(player_balance, amount) {
            set_user_balance(player.clone(), player_balance - amount, db).await?;
        } else {
            mci.create_response(
                ctx,
                serenity::CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(format!(
                            "You can't afford to do that!\nYour balance is {} J-Bucks.",
                            user_balance
                        ))
                        .ephemeral(true),
                ),
            )
            .await?;

            mci.create_response(ctx, serenity::CreateInteractionResponse::Acknowledge)
                .await?;
            continue;
        }

        let button2;
        let button3;
        {
            let mut games = ctx.data().games.lock().unwrap();
            let game = games.get_mut(&id.to_string()).unwrap();
            game.player_joined(mci.user.id.to_string());
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

        mci.create_response(ctx, serenity::CreateInteractionResponse::Acknowledge)
            .await?;
    }

    let button2;
    let button3;
    let prize;
    let winner;
    {
        let mut games = ctx.data().games.lock().unwrap();
        let game = games.remove(&id.to_string()).unwrap();
        winner = game.get_winner().clone();
        button2 = new_player_count_button(game.players.len() as i32);
        button3 = new_pot_counter_button(game.pot);
        prize = game.pot;
    }
    let winner_balance = get_user_balance(winner.clone(), db).await?;
    set_user_balance(winner.clone(), winner_balance + prize, db).await?;
    let winner_id = winner.parse().unwrap();
    a.edit(
        ctx,
        CreateReply::default()
            .content(format!(
                "Game is over, winner is: {}, they won: {} J-Bucks!",
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
    // TODO: see if we can find a nice way to tell the user their balance after they win
    Ok(())
}

pub async fn get_discord_users(
    ctx: Context<'_>,
    user_ids: Vec<String>,
) -> Result<HashMap<String, String>, Error> {
    let mut users = HashMap::new();
    for user_id in user_ids {
        let user = serenity::UserId::new(user_id.parse().unwrap())
            .to_user(ctx)
            .await?;
        dbg!(ctx.guild_id().unwrap());
        let nick = user.nick_in(ctx, ctx.guild_id().unwrap()).await;
        let nick = match nick {
            Some(nick) => nick,
            None => user.name,
        };
        dbg!(&nick);
        users.insert(user_id, nick);
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
    let stmt = ctx
        .data()
        .db
        .call(|conn| {
            let mut stmt = conn.prepare_cached(
                "SELECT id, balance FROM balances ORDER BY balance DESC LIMIT 10",
            )?;
            let people = stmt
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
                .collect::<std::result::Result<Vec<(String, i32)>, rusqlite::Error>>();
            Ok(people)
        })
        .await?
        .unwrap();
    let ids = stmt
        .clone()
        .iter()
        .map(|(k, _v)| k.to_string())
        .collect::<Vec<String>>();
    let players_resolved = get_discord_users(ctx, ids).await?;

    let top = stmt
        .iter()
        .map(|(k, v)| (players_resolved.get(k).unwrap(), v))
        .enumerate()
        .map(|(i, (k, v))| format!("{}: {} with {} J-Bucks!", i + 1, k, v))
        .collect::<Vec<_>>()
        .join("\n");
    if top.is_empty() {
        ctx.say("Nobody has any J-Bucks yet!").await?;
        return Ok(());
    }

    ctx.say(top).await?;
    Ok(())
}

/// Transfer bucks t another player
///
/// Enter `~transfer @John 50` to transfer 50 bucks to John
#[poise::command(prefix_command, slash_command)]
pub async fn transfer(
    ctx: Context<'_>,
    #[description = "Who to send to"] recipient: User,
    #[description = "How much to send"] amount: i32,
) -> Result<(), Error> {
    let sender = ctx.author().id.to_string();
    let sender_balance = get_user_balance(sender.clone(), &ctx.data().db).await?;
    let recipient_id = recipient.id.to_string();
    let recipient_balance = get_user_balance(recipient_id.clone(), &ctx.data().db).await?;
    if user_can_play(sender_balance, amount) {
        set_user_balance(sender.clone(), sender_balance - amount, &ctx.data().db).await?;
        set_user_balance(
            recipient_id.clone(),
            recipient_balance + amount,
            &ctx.data().db,
        )
        .await?;
    } else {
        let reply = {
            CreateReply::default()
                .content(format!(
                    "You can't afford to do that!\nYour balance is {} J-Bucks.",
                    sender_balance
                ))
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("You can't afford to do that".into());
    }
    let reply = {
        CreateReply::default().content(format!(
            "{} sent {} J-Bucks to {}!",
            ctx.author(),
            amount,
            recipient
        ))
    };
    ctx.send(reply).await?;
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
