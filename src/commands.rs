use rand::{seq::SliceRandom, Rng};
use std::{
    fmt::Display,
    time::{self, SystemTime, UNIX_EPOCH},
    vec,
};

use crate::{database::BalanceDatabase, game::CoinGame, game::Game, texts::*, Context, Error};
use poise::{
    serenity_prelude::{
        self as serenity, CreateAllowedMentions, CreateInteractionResponseMessage, User,
    },
    CreateReply,
};

use self::{landedside::LANDEDSIDE, nowinners::NOWINNERS};

#[poise::command(
    prefix_command,
    track_edits,
    slash_command,
    hide_in_help,
    category = "Admin",
    owners_only
)]
pub async fn register(ctx: Context<'_>) -> Result<(), Error> {
    poise::builtins::register_application_commands_buttons(ctx).await?;
    Ok(())
}

pub async fn complete_help<'a>(
    ctx: Context<'a>,
    partial: &'a str,
) -> impl Iterator<Item = serenity::AutocompleteChoice> + 'a {
    let white_listed = [
        "help",
        "balance",
        "leaderboard",
        "give",
        "coingamble",
        "daily",
    ];
    poise::builtins::autocomplete_command(ctx, partial)
        .await
        .filter(move |cmd| white_listed.contains(&cmd.as_str()))
        .map(|cmd| serenity::AutocompleteChoice::new(cmd.to_string(), cmd))
}

/// Show this help menu
#[poise::command(track_edits, slash_command)]
pub async fn help(
    ctx: Context<'_>,
    #[description = "Specific command to show help about"]
    #[autocomplete = "complete_help"]
    command: Option<String>,
) -> Result<(), Error> {
    if let Some(command) = &command {
        if ![
            "help",
            "balance",
            "leaderboard",
            "give",
            "coingamble",
            "daily",
        ]
        .contains(&command.as_str())
        {
            let reply = {
                CreateReply::default()
                    .content("Unknown command!")
                    .ephemeral(true)
            };
            ctx.send(reply).await?;
            return Ok(());
        }
    }
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
#[poise::command(
    slash_command,
    category = "Admin",
    default_member_permissions = "ADMINISTRATOR",
    hide_in_help
)]
pub async fn checkbucks(
    ctx: Context<'_>,
    #[description = "Who to check"] user: serenity::User,
) -> Result<(), Error> {
    let user_id = user.id.to_string();
    let response = ctx.data().db.get_balance(user_id).await?;
    let reply = {
        CreateReply::default()
            .content(format!("{} has {} J-Buck(s)!", user, response,))
            .ephemeral(true)
    };
    ctx.send(reply).await?;
    Ok(())
}

///
/// Have Johnny say something
///
/// Enter `/say <message>` to make Johnny say something
/// ```
/// /say Awoo
/// ```
///
#[poise::command(
    slash_command,
    category = "Admin",
    default_member_permissions = "ADMINISTRATOR",
    hide_in_help
)]
pub async fn say(
    ctx: Context<'_>,
    #[description = "What to say?"] message: String,
) -> Result<(), Error> {
    let reply = { CreateReply::default().content("Success!").ephemeral(true) };
    ctx.send(reply).await?;
    ctx.channel_id().say(ctx, message).await?;
    Ok(())
}

///
/// Check your balance
///
/// Enter `/balance` to check
/// ```
/// /balance
/// ```
#[poise::command(slash_command)]
pub async fn balance(ctx: Context<'_>) -> Result<(), Error> {
    let user_id = ctx.author().id.to_string();
    let response = ctx.data().db.get_balance(user_id).await?;
    let reply = {
        CreateReply::default()
            .content(format!("{} has {} J-Buck(s)!", ctx.author(), response,))
            .ephemeral(true)
    };
    ctx.send(reply).await?;
    Ok(())
}

fn new_bet_button(amount: i32) -> serenity::CreateButton {
    serenity::CreateButton::new("Bet")
        .label(format!("Bet {} J-Buck(s)", amount))
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

fn new_heads_button() -> serenity::CreateButton {
    serenity::CreateButton::new("Heads")
        .label("Heads")
        .style(poise::serenity_prelude::ButtonStyle::Primary)
}
fn new_tails_button() -> serenity::CreateButton {
    serenity::CreateButton::new("Tails")
        .label("Tails")
        .style(poise::serenity_prelude::ButtonStyle::Primary)
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
#[poise::command(
    track_edits,
    slash_command,
    // user_cooldown = 120
    category = "Admin",
    default_member_permissions = "ADMINISTRATOR",
    hide_in_help
    )]
pub async fn gamble(
    ctx: Context<'_>,
    #[description = "amount to play"]
    #[min = 1]
    amount: i32,
) -> Result<(), Error> {
    let game_length = ctx.data().game_length;
    let game_starter = ctx.author().id.to_string();
    let db = &ctx.data().db;
    let user_balance = db.get_balance(game_starter.clone()).await?;
    if !user_can_play(user_balance, amount) {
        let reply = {
            CreateReply::default()
                .content(format!(
                    "You can't afford to do that!\nYour balance is only {} J-Buck(s)",
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
    let time_to_play = game_length;
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
                            "You can't afford to do that!\nYour balance is only {} J-Buck(s)",
                            user_balance
                        ))
                        .ephemeral(true),
                ),
            )
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
    let winner = game.get_winner(&mut ctx.data().rng.lock().unwrap()).clone();
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
                "Game is over, winner is: {}, they won: {} J-Buck(s)!",
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
    let top = balances
        .iter()
        .map(|(k, v)| (format!("<@{}>", k), v))
        .enumerate()
        .map(|(i, (k, v))| format!("{}: {} with {} J-Buck(s)!", i + 1, k, v))
        .collect::<Vec<_>>()
        .join("\n");
    if top.is_empty() {
        ctx.say("Nobody has any J-Bucks yet!").await?;
        return Ok(());
    }

    let reply = {
        CreateReply::default()
            .content(format!("Leaderboard:\n{}", top))
            .allowed_mentions(CreateAllowedMentions::new().empty_users())
    };

    ctx.send(reply).await?;
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
    if recipient.id.to_string() == ctx.author().id.to_string() {
        let reply = {
            CreateReply::default()
                .content("Don't send money to yourself..")
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("You can't afford to do that".into());
    }
    if recipient.bot {
        let reply = {
            CreateReply::default()
                .content("You can't send money to bots..")
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("You can't afford to do that".into());
    }
    let sender = ctx.author().id.to_string();
    let db = &ctx.data().db;
    let sender_balance = ctx.data().db.get_balance(sender.clone()).await?;
    let recipient_id = recipient.id.to_string();
    let recipient_balance = ctx.data().db.get_balance(recipient_id.clone()).await?;
    if !user_can_play(sender_balance, amount) {
        let reply = {
            CreateReply::default()
                .content(format!(
                    "You can't afford to do that!\nYour balance is only {} J-Buck(s)",
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
            "{} sent {} J-Buck(s) to {}!",
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
#[poise::command(
    slash_command,
    category = "Admin",
    hide_in_help,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn remove_bucks(
    ctx: Context<'_>,
    #[description = "Who to remove from"] user: User,
    #[min = 1]
    #[description = "How much to remove"]
    amount: i32,
) -> Result<(), Error> {
    if user.bot {
        let reply = {
            CreateReply::default()
                .content("You can't remove money from bots..")
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("You can't afford to do that".into());
    }
    let user_id = user.id.to_string();
    let user_balance = ctx.data().db.get_balance(user_id.clone()).await?;
    if !user_can_play(user_balance, amount) {
        let reply = {
            CreateReply::default()
                .content(format!(
                    "They can't afford to do that!\n{}'s balance is only {} J-Buck(s)",
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
#[poise::command(
    slash_command,
    category = "Admin",
    hide_in_help,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn fine(
    ctx: Context<'_>,
    #[description = "Who to fine"] user: User,
    #[min = 1]
    #[description = "How much to fine them"]
    amount: i32,
    #[description = "Show that you invoked the command?"] show_caller: Option<bool>,
    #[description = "Reason for fine"] reason: Option<String>,
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

    let msg = match reason {
        Some(r) => format!(
            "{} was fined {} :dollar:!\nReason: \"*{}*\"",
            user, amount, r
        ),
        None => format!("{} was fined {} :dollar:!", user, amount),
    };

    // if show_caller is true, send as a reply
    match show_caller {
        Some(true) => {
            let reply = { CreateReply::default().content(msg) };
            ctx.send(reply).await?;
        }
        _ => {
            // acknowledge the invocation
            ctx.send(CreateReply::default().content("success").ephemeral(true))
                .await?;
            ctx.channel_id().say(ctx, msg).await?;
        }
    }
    Ok(())
}

///
/// award bucks to a player
///
/// Enter `/award <player> <amount>`
/// ```
/// /award @John 50
/// ```
#[poise::command(
    slash_command,
    category = "Admin",
    hide_in_help,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn award(
    ctx: Context<'_>,
    #[description = "Who to award"] user: User,
    #[min = 1]
    #[description = "How much to award"]
    amount: i32,
    #[description = "Show that you invoked the command?"] show_caller: Option<bool>,
    #[description = "Reason for award"] reason: Option<String>,
) -> Result<(), Error> {
    if user.bot {
        let reply = {
            CreateReply::default()
                .content("You can't award bots..")
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("You can't afford to do that".into());
    }
    let user_id = user.id.to_string();
    let user_balance = ctx.data().db.get_balance(user_id.clone()).await?;
    ctx.data()
        .db
        .set_balance(user_id.clone(), user_balance + amount)
        .await?;

    // if show_caller is true, send as a reply
    let msg = match reason {
        Some(m) => format!(
            "{} was awarded {} :dollar:!\nReason: \"*{}*\"",
            user, amount, m
        ),
        None => format!("{} was awarded {} :dollar:!", user, amount),
    };
    match show_caller {
        Some(true) => {
            let reply = { CreateReply::default().content(msg) };
            ctx.send(reply).await?;
        }
        _ => {
            // acknowledge the invocation
            ctx.send(CreateReply::default().content("success").ephemeral(true))
                .await?;
            ctx.channel_id().say(ctx, msg).await?;
        }
    }
    Ok(())
}

///
/// add bucks to a player
///
/// Enter `/add_bucks <player> <amount>`
/// ```
/// /add_bucks @John 50
/// ```
#[poise::command(
    slash_command,
    category = "Admin",
    hide_in_help,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn add_bucks(
    ctx: Context<'_>,
    #[description = "Who to give bucks to"] user: User,
    #[min = 1]
    #[description = "How much to add"]
    amount: i32,
) -> Result<(), Error> {
    if user.bot {
        let reply = {
            CreateReply::default()
                .content("You can't add money to bots..")
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("You can't afford to do that".into());
    }
    let user_id = user.id.to_string();
    let user_balance = ctx.data().db.get_balance(user_id.clone()).await?;
    ctx.data()
        .db
        .set_balance(user_id.clone(), user_balance + amount)
        .await?;
    let reply =
        { CreateReply::default().content(format!("{} was given {} J-Buck(s)", user, amount,)) };
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
#[poise::command(
    slash_command,
    category = "Admin",
    hide_in_help,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn transfer(
    ctx: Context<'_>,
    #[description = "Who to remove from"] source: User,
    #[description = "Who to give to"] recipient: User,
    #[min = 1]
    #[description = "How much to transfer"]
    amount: i32,
) -> Result<(), Error> {
    if source.id == recipient.id {
        let reply = {
            CreateReply::default()
                .content("No action required")
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("No action required".into());
    }
    if source.bot || recipient.bot {
        let reply = {
            CreateReply::default()
                .content("You can't transfer money to or from bots..")
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("You can't afford to do that".into());
    }
    let user_id = source.id.to_string();
    let user_balance = ctx.data().db.get_balance(user_id.clone()).await?;
    if !user_can_play(user_balance, amount) {
        let reply = {
            CreateReply::default()
                .content(format!(
                    "They can't afford to do that!\n{}'s balance is only {} J-Buck(s)",
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
            "Removed {} J-Buck(s) from {} and gave it to {}",
            amount, source, recipient
        ))
    };
    ctx.send(reply).await?;
    Ok(())
}

#[derive(poise::ChoiceParameter, Clone)]
pub enum HeadsOrTail {
    #[name = "Heads"]
    Heads,
    #[name = "Tails"]
    Tails,
}

impl Display for HeadsOrTail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HeadsOrTail::Heads => write!(f, "Heads"),
            HeadsOrTail::Tails => write!(f, "Tails"),
        }
    }
}

///
/// Start a coin gamble
///
/// Enter `/gamble <amount>`
/// ```
/// /coingamble 10
/// ```
#[poise::command(slash_command)]
pub async fn coingamble(
    ctx: Context<'_>,
    #[min = 1]
    #[description = "How much to play"]
    amount: i32,
    choice: HeadsOrTail,
) -> Result<(), Error> {
    let game_length = ctx.data().game_length;
    let db = &ctx.data().db;
    let game_starter = ctx.author().id.to_string();
    let user_balance = ctx.data().db.get_balance(game_starter.clone()).await?;
    if !user_can_play(user_balance, amount) {
        let reply = {
            CreateReply::default()
                .content(format!(
                    "You can't afford to do that!\nYour balance is only {} J-Buck(s)",
                    user_balance
                ))
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("can't afford to do that".into());
    }
    db.set_balance(game_starter.clone(), user_balance - amount)
        .await?;

    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let time_to_play = game_length;
    let pot = amount;
    let components = vec![serenity::CreateActionRow::Buttons(vec![
        new_heads_button(),
        new_tails_button(),
        new_player_count_button(1),
        new_pot_counter_button(pot),
    ])];
    let reply = {
        CreateReply::default()
            .content(format!(
                "> ### :coin: HEADS OR TAILS?\n> **Bet {} :dollar: **on the correct answer!\n> **Game Ends: **<t:{}:R>",
                amount,
                now + time_to_play
            ))
            .components(components.clone())
    };

    let a = ctx.send(reply).await?;
    let id = a.message().await?.id;

    let coingame = CoinGame::new(
        id.to_string(),
        game_starter.clone(),
        choice.clone(),
        amount,
        time::Instant::now(),
        ctx.data().side_chance,
    );

    ctx.data()
        .coingames
        .lock()
        .unwrap()
        .insert(id.to_string(), coingame);

    while let Some(mci) = serenity::ComponentInteractionCollector::new(ctx)
        .channel_id(ctx.channel_id())
        .custom_ids(vec!["Heads".to_string(), "Tails".to_string()])
        .message_id(id)
        .timeout(std::time::Duration::from_secs(
            (now + time_to_play - 1) - SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        ))
        // .filter(move |mci| mci.data.custom_id == "Bet")
        .await
    {
        let player = mci.user.id.to_string();
        {
            if ctx
                .data()
                .coingames
                .lock()
                .unwrap()
                .get(&id.to_string())
                .unwrap()
                .players
                .contains(&player)
            {
                mci.create_response(
                    ctx,
                    serenity::CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("You are already in this game")
                            .ephemeral(true),
                    ),
                )
                .await?;
                continue;
            }
        }
        if ctx.data().locked_balances.lock().unwrap().contains(&player) {
            mci.create_response(
                ctx,
                serenity::CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(
                                "Nice try, but you can't do that while the robbing event is happening. You can play again after.",
                        )
                        .ephemeral(true),
                ),
            )
            .await?;
            continue;
        }
        let player_balance = db.get_balance(player.clone()).await?;

        if !user_can_play(player_balance, amount) {
            mci.create_response(
                ctx,
                serenity::CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(format!(
                            "You can't afford to do that!\nYour balance is only {} J-Buck(s)",
                            player_balance
                        ))
                        .ephemeral(true),
                ),
            )
            .await?;
            continue;
        }
        db.set_balance(player.clone(), player_balance - amount)
            .await?;

        let button2;
        let button3;
        {
            let mut games = ctx.data().coingames.lock().unwrap();
            let game = games.get_mut(&id.to_string()).unwrap();
            game.player_joined(mci.user.id.to_string(), &mci.data.custom_id);
            button2 = new_player_count_button(game.players.len() as i32);
            button3 = new_pot_counter_button(game.pot);
        }

        let mut msg = mci.message.clone();

        msg.edit(
            ctx,
            serenity::EditMessage::new().components(vec![serenity::CreateActionRow::Buttons(
                vec![new_heads_button(), new_tails_button(), button2, button3],
            )]),
        )
        .await?;

        mci.create_response(
            ctx,
            serenity::CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(format!("You have voted for {}", mci.data.custom_id))
                    .allowed_mentions(CreateAllowedMentions::new().empty_users())
                    .ephemeral(true),
            ),
        )
        .await?;
    }

    let game = {
        let mut games = ctx.data().coingames.lock().unwrap();
        games.remove(&id.to_string()).unwrap()
    };

    let reply = {
        let components = vec![serenity::CreateActionRow::Buttons(vec![
            new_heads_button().disabled(true),
            new_tails_button().disabled(true),
            new_player_count_button(game.players.len() as i32),
            new_pot_counter_button(game.pot),
        ])];
        CreateReply::default()
            .content(format!(
                "> ### :coin: HEADS OR TAILS?\n> **Bet {} :dollar: **on the correct answer!\n> **Game is over!**",
                amount
            ))
            .components(components)
    };

    a.edit(ctx, reply).await?;

    let coin_flip_result = game.get_winner(&mut ctx.data().rng.lock().unwrap()).clone();
    let winners = match coin_flip_result.as_str() {
        "heads" => game.heads.clone(),
        "tails" => game.tails.clone(),
        "side" => vec![],
        _ => vec![],
    };

    if coin_flip_result == "side" {
        let emoji = get_troll_emoji(&mut ctx.data().rng.lock().unwrap());
        let text = get_landed_on_side_text(&mut ctx.data().rng.lock().unwrap());
        let reply = { CreateReply::default().content(format!("{} {}", text, emoji)) };
        ctx.send(reply).await?;
        let edit = {
            let components = vec![serenity::CreateActionRow::Buttons(vec![
                new_heads_button().disabled(true),
                new_tails_button().disabled(true),
                new_player_count_button(game.players.len() as i32),
                new_pot_counter_button(game.pot),
            ])];
            CreateReply::default()
            .content(format!(
                "> ### :coin: HEADS OR TAILS?\n> **Bet {} :dollar: **on the correct answer!\n> **Game is over!**",
                amount
            ))
            .components(components)
        };

        a.edit(ctx, edit).await?;
        return Ok(());
    }
    if winners.is_empty() {
        db.award_balances(game.players.clone(), amount).await?;
        let m = get_nobody_won(&mut ctx.data().rng.lock().unwrap());
        let reply = {
            CreateReply::default().content(format!(
                ":coin: **IT WAS {}!**\n{}",
                coin_flip_result.to_uppercase(),
                m
            ))
        };
        ctx.send(reply).await?;
        let edit = {
            let components = vec![serenity::CreateActionRow::Buttons(vec![
                new_heads_button().disabled(true),
                new_tails_button().disabled(true),
                new_player_count_button(game.players.len() as i32),
                new_pot_counter_button(game.pot),
            ])];
            CreateReply::default()
            .content(format!(
                "> ### :coin: HEADS OR TAILS?\n> **Bet {} :dollar: **on the correct answer!\n> **Game is over!**",
                amount
            ))
            .components(components)
        };

        a.edit(ctx, edit).await?;
        return Ok(());
    }

    let chance_of_bonus = (game.players.len() as f32 * 0.5) - 0.5;

    let johnnys_multiplier =
        if ctx.data().rng.lock().unwrap().gen_range(0..100) < chance_of_bonus as i32 {
            2.0
        } else {
            1.0
        };

    let prize = game.pot / winners.len() as i32;
    let prize_with_multiplier = (prize as f32 * johnnys_multiplier) as i32;

    db.award_balances(winners.clone(), prize_with_multiplier)
        .await?;

    let message = {
        let mut picked_heads_users = game
            .heads
            .iter()
            .map(|u| format!("<@{}>", u))
            .collect::<Vec<_>>()
            .join(" ");

        let mut picked_tails_users = game
            .tails
            .iter()
            .map(|u| format!("<@{}>", u))
            .collect::<Vec<_>>()
            .join(" ");

        if picked_heads_users.is_empty() {
            picked_heads_users = "Nobody!".to_string();
        }
        if picked_tails_users.is_empty() {
            picked_tails_users = "Nobody!".to_string();
        }
        if coin_flip_result == "heads" {
            picked_heads_users = format!(
                "> {}\n> <:dogePray1:1186283357210947584> Congrats on {} :dollar:!",
                picked_heads_users, prize
            );

            if johnnys_multiplier > 1.0 && prize_with_multiplier - prize > 0 {
                picked_heads_users = format!(
                    "{} +{} Bonus!",
                    picked_heads_users,
                    prize_with_multiplier - prize
                );
            }
            picked_tails_users = format!(
                "> {}\n> <:dogeCrying:1160530365413330974> So sad.",
                picked_tails_users
            );
        } else {
            picked_heads_users = format!(
                "> {}\n> <:dogeCrying:1160530365413330974> So sad.",
                picked_heads_users
            );
            picked_tails_users = format!(
                "> {}\n> <:dogePray1:1186283357210947584> Congrats on {} :dollar:!",
                picked_tails_users, prize
            );
            if johnnys_multiplier > 1.0 && prize_with_multiplier - prize > 0 {
                picked_tails_users = format!(
                    "{} +{} Bonus!",
                    picked_tails_users,
                    prize_with_multiplier - prize
                );
            }
        }

        let mut a = format!(
            "> ### :coin: IT WAS {}!\n> \n",
            coin_flip_result.to_uppercase()
        );
        a.push_str(&format!("> **Picked Heads**\n{}\n> ", picked_heads_users));

        a.push_str(&format!("\n> **Picked Tails**\n{}\n", picked_tails_users));
        CreateReply::default()
            .content(a)
            .allowed_mentions(CreateAllowedMentions::new().empty_users())
    };
    ctx.send(message).await?;
    Ok(())
}

fn get_troll_emoji(a: &mut rand::rngs::StdRng) -> String {
    let emoji = [
        "<:dogeTroll:1160530414490886264>",
        "<:doge:1160530341681954896>",
    ]
    .choose(a)
    .unwrap()
    .to_string();
    emoji
}

fn get_landed_on_side_text(a: &mut rand::rngs::StdRng) -> String {
    LANDEDSIDE.choose(a).unwrap().to_string()
}
fn get_nobody_won(a: &mut rand::rngs::StdRng) -> String {
    NOWINNERS.choose(a).unwrap().to_string()
}
