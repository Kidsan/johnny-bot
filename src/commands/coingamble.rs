use std::time::{self, SystemTime, UNIX_EPOCH};

use crate::{
    commands::rockpaperscissors::award_role_holder, database::BalanceDatabase, game::CoinGame,
    texts::landedside::LANDEDSIDE, Context, Error,
};
use poise::{serenity_prelude as serenity, CreateReply};
use rand::{seq::SliceRandom, Rng};
///
/// Start a coin gamble
///
/// Enter `/gamble <amount>`
/// ```
/// /coingamble 10
/// ```
#[poise::command(slash_command)]
#[tracing::instrument(level = "info")]
pub async fn coingamble(
    ctx: Context<'_>,
    #[min = 1]
    #[description = "How much to play"]
    amount: i32,
    choice: HeadsOrTail,
) -> Result<(), Error> {
    match minute_cooldown(ctx).await {
        Ok(_) => {}
        Err(e) => return Err(e),
    }
    let game_length = ctx.data().game_length;
    let db = &ctx.data().db;
    let game_starter = ctx.author().id.to_string();
    let user_balance = ctx.data().db.get_balance(game_starter.clone()).await?;
    if amount > user_balance {
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
    db.subtract_balances(vec![game_starter.clone()], amount)
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
                "> ### <:jbuck:1228663982462865450> HEADS OR TAILS?\n> **Bet {} <:jbuck:1228663982462865450> **on the correct answer!\n> **Game Ends: **<t:{}:R>",
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
                        serenity::CreateInteractionResponseMessage::new()
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
                    serenity::CreateInteractionResponseMessage::new()
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

        if amount > player_balance {
            mci.create_response(
                ctx,
                serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::new()
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
        db.subtract_balances(vec![player.clone()], amount).await?;

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
                serenity::CreateInteractionResponseMessage::new()
                    .content(format!("You have voted for {}", mci.data.custom_id))
                    .allowed_mentions(serenity::CreateAllowedMentions::new().empty_users())
                    .ephemeral(true),
            ),
        )
        .await?;
    }

    let mut game = {
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
                "> ### <:jbuck:1228663982462865450> HEADS OR TAILS?\n> **Bet {} <:jbuck:1228663982462865450> **on the correct answer!\n> **Game is over!**",
                amount
            ))
            .components(components)
    };

    a.edit(ctx, reply).await?;

    if game.heads.is_empty() {
        game.heads.push(ctx.data().bot_id.to_string());
        game.players.push(ctx.data().bot_id.to_string());
        game.pot += game.pot;
    } else if game.tails.is_empty() {
        game.tails.push(ctx.data().bot_id.to_string());
        game.players.push(ctx.data().bot_id.to_string());
        game.pot += game.pot;
    }

    let coin_flip_result = game.get_winner(&mut ctx.data().rng.lock().unwrap()).clone();
    tracing::event!(
        tracing::Level::INFO,
        "Coin flip result: {}",
        coin_flip_result
    );
    let winners = match coin_flip_result.as_str() {
        "heads" => game.heads.clone(),
        "tails" => game.tails.clone(),
        "side" => vec![],
        _ => vec![],
    };

    if coin_flip_result == "side" {
        let emoji = get_troll_emoji(&mut ctx.data().rng.lock().unwrap());
        let leaders: Vec<String> = db
            .get_leaderboard()
            .await?
            .iter()
            .map(|(u, _b)| u.to_owned())
            .collect();
        let each = game.pot / leaders.len() as i32;
        let text = match each {
            0 => format!(
                "{} {}",
                get_landed_on_side_text(&mut ctx.data().rng.lock().unwrap()),
                emoji
            ),
            _ => {
                ctx.data().db.award_balances(leaders, each).await?;
                format!("### Woah, a side coin!\n No way to call a winner here, let's split it with everyone on the leaderboard to be fair <:dogeTroll:1160530414490886264> (+ {} <:jbuck:1228663982462865450> to everyone in the top 10)", each)
            }
        };
        let reply = { CreateReply::default().content(text) };
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
                "> ### <:jbuck:1228663982462865450> HEADS OR TAILS?\n> **Bet {} <:jbuck:1228663982462865450> **on the correct answer!\n> **Game is over!**",
                amount
            ))
            .components(components)
        };

        a.edit(ctx, edit).await?;
        return Ok(());
    }
    let chance_of_bonus = game.players.len();

    let johnnys_multiplier = if ctx.data().rng.lock().unwrap().gen_range(0..100) < chance_of_bonus {
        ctx.data().rng.lock().unwrap().gen_range(0.20..=2.0)
    } else {
        0.0
    };

    let prize = game.pot / winners.len() as i32;
    let remainder = game.pot % winners.len() as i32;
    let prize_with_multiplier = prize + (prize as f32 * johnnys_multiplier) as i32;
    let mut leader = "".to_string();

    if winners[0] != ctx.data().bot_id {
        db.award_balances(winners.clone(), prize_with_multiplier)
            .await?;
        if remainder > 0 {
            leader = if let Some(user) = award_role_holder(ctx, remainder).await? {
                user
            } else {
                "".to_string()
            };
        }
    }

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
                "> {}\n> <:dogePray1:1186283357210947584> Congrats on {} <:jbuck:1228663982462865450>!",
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
                "> {}\n> <:dogePray1:1186283357210947584> Congrats on {} <:jbuck:1228663982462865450>!",
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
            "> ### <:jbuck:1228663982462865450> IT WAS {}!\n> \n",
            coin_flip_result.to_uppercase()
        );
        a.push_str(&format!("> **Picked Heads**\n{}\n> ", picked_heads_users));

        a.push_str(&format!("\n> **Picked Tails**\n{}\n", picked_tails_users));

        if remainder > 0 {
            a.push_str(&format!(
                "> \n> +{} <:jbuck:1228663982462865450> to <@{}> ||(Crown's Tax)||",
                remainder, leader
            ));
        }

        CreateReply::default()
            .content(a)
            .allowed_mentions(serenity::CreateAllowedMentions::new().empty_users())
    };
    ctx.send(message).await?;
    Ok(())
}

#[derive(poise::ChoiceParameter, Clone, Debug)]
pub enum HeadsOrTail {
    #[name = "Heads"]
    Heads,
    #[name = "Tails"]
    Tails,
}

impl std::fmt::Display for HeadsOrTail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HeadsOrTail::Heads => write!(f, "Heads"),
            HeadsOrTail::Tails => write!(f, "Tails"),
        }
    }
}

pub(crate) fn new_player_count_button(amount: i32) -> serenity::CreateButton {
    serenity::CreateButton::new("Players")
        .label(format!("Players: {} ", amount))
        .disabled(true)
        .style(poise::serenity_prelude::ButtonStyle::Secondary)
}
pub(crate) fn new_pot_counter_button(amount: i32) -> serenity::CreateButton {
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
fn get_landed_on_side_text(a: &mut rand::rngs::StdRng) -> String {
    LANDEDSIDE.choose(a).unwrap().to_string()
}

pub fn get_troll_emoji(a: &mut rand::rngs::StdRng) -> String {
    let emoji = [
        "<:dogeTroll:1160530414490886264>",
        "<:doge:1160530341681954896>",
    ]
    .choose(a)
    .unwrap()
    .to_string();
    emoji
}

async fn minute_cooldown(ctx: Context<'_>) -> Result<(), Error> {
    let mut remains = time::Duration::from_secs(0);
    let proceed = {
        let mut cooldown_tracker = ctx.command().cooldowns.lock().unwrap();

        let cooldown_durations = poise::CooldownConfig {
            user: Some(time::Duration::from_secs(ctx.data().game_length)),
            ..Default::default()
        };

        match cooldown_tracker.remaining_cooldown(ctx.cooldown_context(), &cooldown_durations) {
            Some(remaining) => {
                let var_name = false;
                remains = remaining;
                var_name
            }
            None => {
                cooldown_tracker.start_cooldown(ctx.cooldown_context());
                true
            }
        }
    };
    if !proceed {
        let reply = {
            CreateReply::default()
                .content(format!(
                    "You can use this command again in {} seconds",
                    remains.as_secs()
                ))
                .ephemeral(true)
        };
        ctx.send(reply).await.unwrap();
        return Err("You can use this command again in 60 seconds".into());
    }
    Ok(())
}
