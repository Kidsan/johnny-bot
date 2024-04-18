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
    // let db = &ctx.data().db;
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
                "> ### It's Blackjack time, roll the :game_die: to play!\n> **You have <t:{}:R> seconds to play.**",
                now + time_to_play
            ))
            .components(components.clone())
    };

    let a = ctx.send(reply).await?;
    let id = a.message().await?.id;
    ctx.serenity_context()
        .shard
        .set_activity(Some(serenity::ActivityData::playing("Blackjack!")));

    let mut blackjack = Blackjack::new(id.to_string(), game_starter);

    // ctx.data()
    //     .coingames
    //     .lock()
    //     .unwrap()
    //     .insert(id.to_string(), blackjack);

    while let Some(mci) = serenity::ComponentInteractionCollector::new(ctx)
        .channel_id(ctx.channel_id())
        .custom_ids(vec!["twodice".to_string(), "onedice".to_string()])
        // .message_id(id)
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

        if blackjack.players_scores[idx] >= 21 {
            msg = format!(
                "You already have a score of {}.\nYou can't roll anymore.",
                blackjack.players_scores[idx]
            );
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
            if blackjack.players_scores[idx] < 16 {
                msg = "You can't roll a single dice until your score is 16 or higher.".to_string();
                mci.create_response(
                    ctx,
                    serenity::CreateInteractionResponse::Message(
                        serenity::CreateInteractionResponseMessage::new()
                            .content(msg)
                            .allowed_mentions(serenity::CreateAllowedMentions::new().empty_users())
                            .components(components.clone())
                            .ephemeral(true),
                    ),
                )
                .await?;
                continue;
            }
            let total = ctx.data().rng.lock().unwrap().gen_range(1..=6);
            blackjack.players_scores[idx] += total;
            msg = format!(
                "You rolled a {}.\nYour current score is {}.",
                total, blackjack.players_scores[idx]
            )
        }

        if blackjack.players_scores[idx] > 21 {
            msg += "\nYou busted!";
        }

        let components = match blackjack.players_scores[idx] >= 16 {
            true => vec![serenity::CreateActionRow::Buttons(vec![
                new_twodice_button(),
                new_onedice_button(),
                new_player_count_button(blackjack.players.len() as i32),
                new_pot_counter_button(0),
            ])],
            false => vec![serenity::CreateActionRow::Buttons(vec![
                new_twodice_button(),
                new_player_count_button(blackjack.players.len() as i32),
                new_pot_counter_button(0),
            ])],
        };

        mci.create_response(
            ctx,
            serenity::CreateInteractionResponse::Message(
                serenity::CreateInteractionResponseMessage::new()
                    .content(msg)
                    .components(components.clone())
                    .allowed_mentions(serenity::CreateAllowedMentions::new().empty_users())
                    .ephemeral(true),
            ),
        )
        .await?;
    }

    // let mut game = {
    //     let mut games = ctx.data().coingames.lock().unwrap();
    //     games.remove(&id.to_string()).unwrap()
    // };

    let reply = {
        let components = vec![serenity::CreateActionRow::Buttons(vec![
            new_twodice_button().disabled(true),
            new_onedice_button().disabled(true),
            new_player_count_button(blackjack.players.len() as i32),
            new_pot_counter_button(0),
        ])];
        CreateReply::default()
            .content(format!(
                "> ### <:jbuck:1228663982462865450> HEADS OR TAILS?\n> **Bet {} <:jbuck:1228663982462865450> **on the correct answer!\n> **Game is over!**",
                0
            ))
            .components(components)
    };

    a.edit(ctx, reply).await?;
    ctx.serenity_context().shard.set_activity(None);
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
