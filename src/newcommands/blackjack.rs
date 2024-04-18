use std::time::{SystemTime, UNIX_EPOCH};

use crate::{database::BalanceDatabase, game::Blackjack, Context, Error};
use poise::{serenity_prelude as serenity, CreateReply};
use rand::{seq::SliceRandom, Rng};
///
/// Start a blackjack game
///
/// Enter `/blackjack`
/// ```
/// /blackjack
/// ```
#[poise::command(slash_command)]
#[tracing::instrument(level = "info")]
pub async fn blackjack(ctx: Context<'_>) -> Result<(), Error> {
    let game_length = ctx.data().game_length;
    let db = &ctx.data().db;
    let game_starter = ctx.author().id.to_string();
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let time_to_play = game_length;
    let components = vec![serenity::CreateActionRow::Buttons(vec![
        new_twodice_button(),
        new_player_count_button(1),
    ])];
    let reply = {
        CreateReply::default()
            .content(format!(
                "> ### <:jbuck:1228663982462865450> HEADS OR TAILS?\n> **Bet {} <:jbuck:1228663982462865450> **on the correct answer!\n> **Game Ends: **<t:{}:R>",
                0,
                now + time_to_play
            ))
            .components(components.clone())
    };

    let a = ctx.send(reply).await?;
    let id = a.message().await?.id;

    let mut blackjack = Blackjack::new(id.to_string(), game_starter);

    // ctx.data()
    //     .coingames
    //     .lock()
    //     .unwrap()
    //     .insert(id.to_string(), blackjack);

    while let Some(mci) = serenity::ComponentInteractionCollector::new(ctx)
        .channel_id(ctx.channel_id())
        .custom_ids(vec!["twodice".to_string(), "onedice".to_string()])
        .message_id(id)
        .timeout(std::time::Duration::from_secs(
            (now + time_to_play - 1) - SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        ))
        .await
    {
        let player = mci.user.id.to_string();
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

        let mut msg = String::new();

        if !blackjack.players.contains(&player) {
            blackjack.player_joined(player.clone());
        }
        let idx = blackjack
            .players
            .iter()
            .enumerate()
            .find(|x| x.1 == &player)
            .unwrap()
            .0;

        if mci.data.custom_id == "twodice" {
            let one = ctx.data().rng.lock().unwrap().gen_range(1..=6);
            let two = ctx.data().rng.lock().unwrap().gen_range(1..=6);
            let total = one + two;
            blackjack.players_scores[idx] += total;
            msg = format!(
                "You rolled a {} and a {} for a total of {}.\nYour current score is {}.",
                one, two, total, blackjack.players_scores[idx]
            )
        } else if mci.data.custom_id == "onedice" {
            let total = ctx.data().rng.lock().unwrap().gen_range(1..=6);
            blackjack.players_scores[idx] += total;
            msg = format!(
                "You rolled a {}.\nYour current score is {}.",
                total, blackjack.players_scores[idx]
            )
        }

        if blackjack.players_scores[idx] > 21 {
            msg = format!(
                "You busted with a score of {}.",
                blackjack.players_scores[idx]
            );
        }

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
            new_twodice_button().disabled(true),
            new_twodice_button().disabled(true),
            new_player_count_button(game.players.len() as i32),
            new_pot_counter_button(game.pot),
        ])];
        CreateReply::default()
            .content(format!(
                "> ### <:jbuck:1228663982462865450> HEADS OR TAILS?\n> **Bet {} <:jbuck:1228663982462865450> **on the correct answer!\n> **Game is over!**",
                0
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
            0 => format!("{} {}", "", emoji),
            _ => {
                ctx.data().db.award_balances(leaders, each).await?;
                format!("### Woah, a side coin!\n No way to call a winner here, let's split it with everyone on the leaderboard to be fair <:dogeTroll:1160530414490886264> (+ {} <:jbuck:1228663982462865450> to everyone in the top 10)", each)
            }
        };
        let reply = { CreateReply::default().content(text) };
        ctx.send(reply).await?;
        let edit = {
            let components = vec![serenity::CreateActionRow::Buttons(vec![
                new_twodice_button().disabled(true),
                new_twodice_button().disabled(true),
                new_player_count_button(game.players.len() as i32),
                new_pot_counter_button(game.pot),
            ])];
            CreateReply::default()
            .content(format!(
                "> ### <:jbuck:1228663982462865450> HEADS OR TAILS?\n> **Bet {} <:jbuck:1228663982462865450> **on the correct answer!\n> **Game is over!**",
                0
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
            leader = ctx.data().db.get_leader().await?;
            db.award_balances(vec![leader.clone()], remainder).await?;
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
                "> \n> +{} <:jbuck:1228663982462865450> to <@{}> ||(leader bonus)||",
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

fn new_onedice_button() -> serenity::CreateButton {
    serenity::CreateButton::new("onedice")
        .label("Roll One Dice")
        .style(poise::serenity_prelude::ButtonStyle::Secondary)
}

fn new_twodice_button() -> serenity::CreateButton {
    serenity::CreateButton::new("twodice")
        .label("Roll Two Dice")
        .style(poise::serenity_prelude::ButtonStyle::Primary)
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
