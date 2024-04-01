use std::{
    collections::HashMap,
    time::{self, SystemTime, UNIX_EPOCH},
    vec,
};

use crate::{database::BalanceDatabase, game::Game, Context, Error};
use poise::{
    serenity_prelude::{self as serenity, CreateInteractionResponseMessage, User},
    CreateReply,
};

#[poise::command(
    prefix_command,
    track_edits,
    slash_command,
    hide_in_help,
    category = "Admin"
)]
pub async fn register(ctx: Context<'_>) -> Result<(), Error> {
    poise::builtins::register_application_commands_buttons(ctx).await?;
    Ok(())
}

/// Show this help menu
#[poise::command(track_edits, slash_command)]
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
            extra_text_at_bottom: "Awooo",
            ..Default::default()
        },
    )
    .await?;
    Ok(())
}

///
/// Check someone's balance
///
/// Enter `/checkbucks @Name` to check
/// ```
/// /checkbucks @John
/// ```
///
#[poise::command(slash_command, category = "Admin")]
pub async fn checkbucks(
    ctx: Context<'_>,
    #[description = "Who to check"] user: serenity::User,
) -> Result<(), Error> {
    let user_id = user.id.to_string();
    let response = ctx.data().db.get_balance(user_id).await?;
    let reply = {
        CreateReply::default()
            .content(format!("{} has {} J-Bucks!", ctx.author(), response,))
            .ephemeral(true)
    };
    ctx.send(reply).await?;
    Ok(())
}

///
/// Check your balance
///
/// Enter `/checkbucks` to check
/// ```
/// /checkbucks
/// ```
#[poise::command(slash_command)]
pub async fn balance(ctx: Context<'_>) -> Result<(), Error> {
    let user_id = ctx.author().id.to_string();
    let response = ctx.data().db.get_balance(user_id).await?;
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

///
/// Start a gamble
///
/// Enter `/gamble <amount>` to play
/// ```
/// /gamble 20
/// ```
#[poise::command(track_edits, slash_command)]
pub async fn gamble(
    ctx: Context<'_>,
    #[description = "amount to play"]
    #[min = 1]
    amount: i32,
) -> Result<(), Error> {
    let game_starter = ctx.author().id.to_string();
    let db = &ctx.data().db;
    let user_balance = db.get_balance(game_starter.clone()).await?;
    if !user_can_play(user_balance, amount) {
        let reply = {
            CreateReply::default()
                .content(format!(
                    "You can't afford to do that!\nYour balance is only {} J-Bucks",
                    user_balance
                ))
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("You can't afford to do that".into());
    }
    db.set_balance(game_starter.clone(), user_balance - amount)
        .await?;

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

        let player_balance = db.get_balance(player.clone()).await?;

        if !user_can_play(player_balance, amount) {
            mci.create_response(
                ctx,
                serenity::CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(format!(
                            "You can't afford to do that!\nYour balance is only {} J-Bucks",
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
        db.set_balance(player.clone(), player_balance - amount)
            .await?;

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

    let game = {
        let mut games = ctx.data().games.lock().unwrap();
        games.remove(&id.to_string()).unwrap()
    };
    let winner = game.get_winner().clone();
    let button2 = new_player_count_button(game.players.len() as i32);
    let button3 = new_pot_counter_button(game.pot);
    let prize = game.pot;

    let winner_balance = db.get_balance(winner.clone()).await?;
    db.set_balance(winner.clone(), winner_balance + prize)
        .await?;
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
        let nick = user.nick_in(ctx, ctx.guild_id().unwrap()).await;
        let nick = match nick {
            Some(nick) => nick,
            None => user.name,
        };
        users.insert(user_id, nick);
    }
    Ok(users)
}

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
    let ids = balances
        .clone()
        .iter()
        .map(|(k, _v)| k.to_string())
        .collect::<Vec<String>>();
    let players_resolved = get_discord_users(ctx, ids).await?;

    let top = balances
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

///
/// Give some bucks to another player
///
/// Enter `/give <recipient> <amount>`
/// ```
/// /give @John 50
/// ```
#[poise::command(slash_command)]
pub async fn give(
    ctx: Context<'_>,
    #[description = "Who to send to"] recipient: User,
    #[min = 1]
    #[description = "How much to send"]
    amount: i32,
) -> Result<(), Error> {
    let sender = ctx.author().id.to_string();
    let db = &ctx.data().db;
    let sender_balance = ctx.data().db.get_balance(sender.clone()).await?;
    let recipient_id = recipient.id.to_string();
    let recipient_balance = ctx.data().db.get_balance(recipient_id.clone()).await?;
    if !user_can_play(sender_balance, amount) {
        let reply = {
            CreateReply::default()
                .content(format!(
                    "You can't afford to do that!\nYour balance is only {} J-Bucks",
                    sender_balance
                ))
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("You can't afford to do that".into());
    }
    db.set_balance(sender.clone(), sender_balance - amount)
        .await?;
    db.set_balance(recipient_id.clone(), recipient_balance + amount)
        .await?;
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

///
/// Remove bucks from a player
///
/// ```
/// /remove_bucks @John 50
/// ```
#[poise::command(slash_command, category = "Admin")]
pub async fn remove_bucks(
    ctx: Context<'_>,
    #[description = "Who to remove from"] user: User,
    #[min = 1]
    #[description = "How much to remove"]
    amount: i32,
) -> Result<(), Error> {
    let user_id = user.id.to_string();
    let user_balance = ctx.data().db.get_balance(user_id.clone()).await?;
    if !user_can_play(user_balance, amount) {
        let reply = {
            CreateReply::default()
                .content(format!(
                    "They can't afford to do that!\n{}'s balance is only {} J-Bucks",
                    user, user_balance
                ))
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("can't afford to do that".into());
    }
    ctx.data()
        .db
        .set_balance(user_id.clone(), user_balance - amount)
        .await?;

    let reply =
        { CreateReply::default().content(format!("Removed {} J-Bucks from {}", amount, user,)) };
    ctx.send(reply).await?;
    Ok(())
}

///
/// Fine a player
///
/// Enter `/fine <player> <amount>`
/// ```
/// /fine @John 50
/// ```
///
#[poise::command(slash_command, category = "Admin")]
pub async fn fine(
    ctx: Context<'_>,
    #[description = "Who to fine"] user: User,
    #[min = 1]
    #[description = "How much to fine them"]
    amount: i32,
) -> Result<(), Error> {
    let user_id = user.id.to_string();
    let user_balance = ctx.data().db.get_balance(user_id.clone()).await?;
    if !user_can_play(user_balance, amount) {
        let reply = {
            CreateReply::default()
                .content(format!(
                    "They can't afford to do that!\n{}'s balance is only {} J-Bucks",
                    user, user_balance
                ))
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("Can't afford to do that".into());
    }
    ctx.data()
        .db
        .set_balance(user_id.clone(), user_balance - amount)
        .await?;

    let reply = {
        CreateReply::default().content(format!(
            "{} was fined {} J-Bucks {}",
            user,
            amount,
            ctx.guild()
                .unwrap()
                .emojis
                .get(&serenity::EmojiId::new(548288157095952394))
                .unwrap()
        ))
    };
    ctx.send(reply).await?;
    Ok(())
}

///
/// award bucks to a player
///
/// Enter `/award <player> <amount>`
/// ```
/// /award @John 50
/// ```
#[poise::command(slash_command, category = "Admin")]
pub async fn award(
    ctx: Context<'_>,
    #[description = "Who to award"] user: User,
    #[min = 1]
    #[description = "How much to award"]
    amount: i32,
) -> Result<(), Error> {
    let user_id = user.id.to_string();
    let user_balance = ctx.data().db.get_balance(user_id.clone()).await?;
    ctx.data()
        .db
        .set_balance(user_id.clone(), user_balance + amount)
        .await?;
    let reply =
        { CreateReply::default().content(format!("{} was awarded {} J-Bucks", user, amount,)) };
    ctx.send(reply).await?;
    Ok(())
}

///
/// add bucks to a player
///
/// Enter `/add_bucks <player> <amount>`
/// ```
/// /add_bucks @John 50
/// ```
#[poise::command(slash_command, category = "Admin")]
pub async fn add_bucks(
    ctx: Context<'_>,
    #[description = "Who to give bucks to"] user: User,
    #[min = 1]
    #[description = "How much to add"]
    amount: i32,
) -> Result<(), Error> {
    let user_id = user.id.to_string();
    let user_balance = ctx.data().db.get_balance(user_id.clone()).await?;
    ctx.data()
        .db
        .set_balance(user_id.clone(), user_balance + amount)
        .await?;
    let reply =
        { CreateReply::default().content(format!("{} was given {} J-Bucks", user, amount,)) };
    ctx.send(reply).await?;
    Ok(())
}

///
/// Transfer some bucks between players
///
/// Enter `/transfer <source> <recipient> <amount>`
/// ```
/// /transfer @John @Adam 50
/// ```
#[poise::command(slash_command, category = "Admin")]
pub async fn transfer(
    ctx: Context<'_>,
    #[description = "Who to remove from"] source: User,
    #[description = "Who to give to"] recipient: User,
    #[min = 1]
    #[description = "How much to transfer"]
    amount: i32,
) -> Result<(), Error> {
    let user_id = source.id.to_string();
    let user_balance = ctx.data().db.get_balance(user_id.clone()).await?;
    if !user_can_play(user_balance, amount) {
        let reply = {
            CreateReply::default()
                .content(format!(
                    "They can't afford to do that!\n{}'s balance is only {} J-Bucks",
                    source, user_balance
                ))
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("can't afford to do that".into());
    }
    let recipient_id = recipient.id.to_string();
    let recipient_balance = ctx.data().db.get_balance(recipient_id.clone()).await?;
    ctx.data()
        .db
        .set_balance(user_id.clone(), user_balance - amount)
        .await?;
    ctx.data()
        .db
        .set_balance(recipient_id.clone(), recipient_balance + amount)
        .await?;

    let reply = {
        CreateReply::default().content(format!(
            "Removed {} J-Bucks from {} and gave it to {}",
            amount, source, recipient
        ))
    };
    ctx.send(reply).await?;
    Ok(())
}
