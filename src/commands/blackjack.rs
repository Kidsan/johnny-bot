use std::cmp::Ordering;
use std::fmt::Write;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::database::BalanceDatabase;
use crate::{game::Blackjack, Context, Error};
use poise::{serenity_prelude as serenity, CreateReply, ReplyHandle};
use rand::{seq::SliceRandom, Rng};

async fn in_blackjack(ctx: Context<'_>) -> Result<bool, Error> {
    if *ctx.data().blackjack_active.lock().unwrap() {
        let reply = {
            CreateReply::default()
                .content("There is already a blackjack game running.")
                .ephemeral(true)
                .allowed_mentions(serenity::CreateAllowedMentions::new().empty_users())
        };
        let _ = ctx.send(reply).await;
        return Ok(false);
    }
    Ok(true)
}

///
/// Start a blackjack game
///
/// Enter `/blackjack`
/// ```
/// /blackjack
/// ```
#[poise::command(slash_command, check = "in_blackjack")]
#[tracing::instrument(level = "info")]
pub async fn blackjack(
    ctx: Context<'_>,
    #[description = "Amount to bet"]
    #[min = 1]
    #[max = 5]
    amount: i32,
) -> Result<(), Error> {
    {
        *ctx.data().blackjack_active.lock().unwrap() = true;
    }
    let game_length = ctx.data().game_length;
    let db = &ctx.data().db;
    // let game_starter = ctx.author().id.to_string();
    let player_balance = db
        .get_balance(ctx.author().id.get().try_into().unwrap())
        .await?;
    if player_balance < amount {
        let reply = {
            CreateReply::default()
                .content("You don't have enough to play.")
                .ephemeral(true)
                .allowed_mentions(serenity::CreateAllowedMentions::new().empty_users())
        };
        ctx.send(reply).await?;
        {
            *ctx.data().blackjack_active.lock().unwrap() = false;
        }
        return Ok(());
    }

    let start_time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let time_to_play = game_length;
    let reply = {
        CreateReply::default()
            .content(format!(
                "> ### It's Blackjack time, roll the :game_die: to play!\n{}> **You have <t:{}:R> seconds to play.**",
                "",
                start_time + time_to_play
            ))
            .components(

                vec![serenity::CreateActionRow::Buttons(vec![
                    new_twodice_button(),
                    new_player_count_button(1),
                ])]
            )
    };

    let a = ctx.send(reply).await?;
    let id = a.message().await?.id;
    ctx.serenity_context()
        .shard
        .set_activity(Some(serenity::ActivityData::playing("Blackjack!")));

    let game = Mutex::new(Blackjack::new(id.to_string()));
    let bot_idx = {
        let mut g = game.lock().unwrap();
        g.player_joined(ctx.data().bot_id.clone());
        g.pot += amount * 2;
        g.players
            .iter()
            .enumerate()
            .find(|x| x.1 == &ctx.data().bot_id)
            .unwrap()
            .0
    };

    while let Some(mci) = serenity::ComponentInteractionCollector::new(ctx)
        .channel_id(ctx.channel_id())
        .custom_ids(vec![
            "twodice".to_string(),
            "onedice".to_string(),
            "stand".to_string(),
        ])
        .timeout(std::time::Duration::from_secs(
            (start_time + time_to_play - 1)
                - SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        ))
        .await
    {
        let player = mci.user.id.to_string();
        if ctx
            .data()
            .locked_balances
            .lock()
            .unwrap()
            .contains(&(mci.user.id.get() as i64))
        {
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
        if mci.data.custom_id == "stand" {
            mci.create_response(ctx, serenity::CreateInteractionResponse::Acknowledge)
                .await?;
            if mci.message.id != id {
                mci.delete_followup(ctx, mci.message.id).await?;
            }
            continue;
        }

        let mut msg = String::new();

        let new_player: bool = {
            let game = game.lock().unwrap();
            !game.players.contains(&player)
        };
        if new_player {
            let player_balance = db
                .get_balance(mci.user.id.get().try_into().unwrap())
                .await?;
            if player_balance < amount {
                let reply = {
                    CreateReply::default()
                        .content("You don't have enough to play.")
                        .ephemeral(true)
                        .allowed_mentions(serenity::CreateAllowedMentions::new().empty_users())
                };
                ctx.send(reply).await?;
                continue;
            }
            game.lock().unwrap().player_joined(player.clone());
            db.subtract_balances(vec![player.parse().unwrap()], amount)
                .await?;
            game.lock().unwrap().pot += amount;
        }

        let idx = {
            game.lock()
                .unwrap()
                .players
                .iter()
                .enumerate()
                .find(|x| x.1 == &player)
                .unwrap()
                .0
        };

        {
            if game.lock().unwrap().players_scores[idx] >= 21 {
                msg = format!(
                    "You already have a score of {}.\nYou can't roll anymore.",
                    game.lock().unwrap().players_scores[idx]
                );
                if mci.message.id != id {
                    mci.delete_followup(ctx, mci.message.id).await?;
                }
                mci.create_response(
                    ctx,
                    serenity::CreateInteractionResponse::Message(
                        serenity::CreateInteractionResponseMessage::new()
                            .content(msg)
                            .allowed_mentions(serenity::CreateAllowedMentions::new().empty_users())
                            .ephemeral(true),
                    ),
                )
                .await?;
                continue;
            }
        }

        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

        if mci.data.custom_id == "twodice" {
            let one = ctx.data().rng.lock().unwrap().gen_range(1..=6);
            let two = ctx.data().rng.lock().unwrap().gen_range(1..=6);
            let total = one + two;
            let mut game = game.lock().unwrap();
            game.players_scores[idx] += total;
            msg = format!(
                "You rolled a {} and a {} for a total of {}.\nYour current score is {}.\nGame Ends: <t:{}:R>",
                one, two, total, game.players_scores[idx], now + (start_time + time_to_play - now)
            );
        } else if mci.data.custom_id == "onedice" {
            let total = ctx.data().rng.lock().unwrap().gen_range(1..=6);
            let mut game = game.lock().unwrap();
            game.players_scores[idx] += total;
            msg = format!(
                "You rolled a {}.\nYour current score is {}.\nGame Ends: <t:{}:R>",
                total,
                game.players_scores[idx],
                now + (start_time + time_to_play - now)
            );
        }

        let mut components = {
            let g = game.lock().unwrap();
            match g.players_scores[idx] >= 16 {
                true => vec![serenity::CreateActionRow::Buttons(vec![
                    new_twodice_button(),
                    new_onedice_button(),
                    new_hold_button(),
                    new_player_count_button(g.players.len() as i32),
                    new_pot_counter_button(g.pot),
                ])],
                false => vec![serenity::CreateActionRow::Buttons(vec![
                    new_twodice_button(),
                    new_hold_button(),
                    new_player_count_button(g.players.len() as i32),
                    new_pot_counter_button(g.pot),
                ])],
            }
        };

        match game.lock().unwrap().players_scores[idx].cmp(&21) {
            Ordering::Greater => {
                msg += "\nYou busted!";
                components = vec![];
            }
            Ordering::Equal => {
                msg += "\nYou got a blackjack!";
                components = vec![];
            }
            _ => {}
        }

        mci.create_response(ctx, serenity::CreateInteractionResponse::Acknowledge)
            .await?;
        if mci.message.id != id {
            mci.delete_followup(ctx, mci.message.id).await?;
        }

        mci.create_followup(
            ctx,
            serenity::CreateInteractionResponseFollowup::new()
                .content(msg)
                .ephemeral(true)
                .components(components),
        )
        .await?;

        {
            update_bot_score(&ctx, &mut game.lock().unwrap());
        };
        let g = { game.lock().unwrap().clone() };
        update_parent_message(&ctx, &a, &g, now + (start_time + time_to_play - now)).await?;
    }

    while game.lock().unwrap().players_scores[bot_idx] < 18 {
        update_bot_score(&ctx, &mut game.lock().unwrap());
    }

    let g = { game.lock().unwrap().clone() };
    update_parent_message(&ctx, &a, &g, 0).await?;

    let winners = game.lock().unwrap().get_winners();
    let prize = match !winners.is_empty() {
        true => game.lock().unwrap().pot / winners.len() as i32,
        false => 0,
    };
    ctx.data()
        .db
        .award_balances(winners.iter().map(|x| x.parse().unwrap()).collect(), prize)
        .await?;
    let losers = g
        .players
        .iter()
        .filter(|x| !winners.contains(x))
        .clone()
        .collect::<Vec<&String>>();

    let reply = {
        CreateReply::default().content(format!(
            "> ### The game is over!\n{}\n{}",
            if !winners.is_empty() {
                format!(
                    "> The winners are: {}\n> Congrats on {} <:jbuck:1228663982462865450>!",
                    winners.iter().fold(String::new(), |mut output, x| {
                        let _ = write!(output, "<@{}> ", x);
                        output
                    }),
                    prize
                )
            } else {
                "> There were no winners.".to_string()
            },
            if !losers.is_empty() {
                format!(
                    "> The losers are: {} {}!",
                    losers.iter().fold(String::new(), |mut output, x| {
                        let _ = write!(output, "<@{}> ", x);
                        output
                    }),
                    get_troll_emoji()
                )
            } else {
                "".to_string()
            }
        ))
    };

    ctx.send(reply).await?;
    ctx.serenity_context().shard.set_activity(None);
    {
        *ctx.data().blackjack_active.lock().unwrap() = false;
    }
    Ok(())
}

fn update_bot_score(ctx: &Context<'_>, game: &mut Blackjack) {
    let idx = game
        .players
        .iter()
        .enumerate()
        .find(|x| x.1 == &ctx.data().bot_id)
        .unwrap()
        .0;

    let bot_score = game.players_scores[idx];

    if bot_score >= 21 {
        return;
    }

    if bot_score < 17 {
        let one = ctx.data().rng.lock().unwrap().gen_range(1..=6);
        let two = ctx.data().rng.lock().unwrap().gen_range(1..=6);
        game.players_scores[idx] += one + two;
    } else if bot_score < 19 || ctx.data().rng.lock().unwrap().gen_range(0..=1) == 1 {
        let one = ctx.data().rng.lock().unwrap().gen_range(1..=6);
        game.players_scores[idx] += one;
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
        .emoji(serenity::ReactionType::Unicode("🎲".to_string()))
}

fn new_twodice_button() -> serenity::CreateButton {
    serenity::CreateButton::new("twodice")
        .label("Roll Two Dice")
        .style(poise::serenity_prelude::ButtonStyle::Primary)
        .emoji(serenity::ReactionType::Unicode("🎲".to_string()))
}

fn new_hold_button() -> serenity::CreateButton {
    serenity::CreateButton::new("stand")
        .label("Stand")
        .style(poise::serenity_prelude::ButtonStyle::Danger)
}

pub fn get_troll_emoji() -> String {
    let emoji = [
        "<:dogeTroll:1160530414490886264>",
        "<:doge:1160530341681954896>",
    ]
    .choose(&mut rand::thread_rng())
    .unwrap()
    .to_string();
    emoji
}

async fn update_parent_message(
    ctx: &Context<'_>,
    msg: &ReplyHandle<'_>,
    game: &Blackjack,
    deadline: u64,
) -> Result<(), Error> {
    let components = vec![serenity::CreateActionRow::Buttons(vec![
        new_twodice_button(),
        new_player_count_button(game.players.len() as i32),
    ])];
    let leaderboard_msg = game
        .get_leaderboard()
        .iter()
        .map(|(id, score)| format!("> <@{}> has a score of {}", id, score))
        .collect::<Vec<String>>()
        .join("\n");
    let reply = {
        CreateReply::default()
            .content(format!(
                "> ### It's Blackjack time, roll the :game_die: to play!\n{}{}",
                leaderboard_msg + "\n",
                if deadline > 0 {
                    format!("> **Game Ends <t:{}:R>**", deadline)
                } else {
                    "> **Game is over!**".to_string()
                }
            ))
            .components(components)
            .allowed_mentions(serenity::CreateAllowedMentions::new().empty_users())
    };

    msg.edit(*ctx, reply).await?;
    Ok(())
}
