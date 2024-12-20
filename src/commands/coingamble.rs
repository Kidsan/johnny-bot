use std::{
    path::PathBuf,
    time::{self, SystemTime, UNIX_EPOCH},
};

use crate::{
    database::BalanceDatabase,
    discord::{
        DOGE_CRY_EMOJI, DOGE_PRAY_EMOJI, DOGE_TROLL_EMOJI_1, DOGE_TROLL_EMOJI_2, JBUCK_EMOJI,
    },
    game::{CoinGame, CoinSides, GameError},
    texts::landedside::LANDEDSIDE,
    Context, Error,
};
use poise::{serenity_prelude as serenity, CreateReply};
use rand::seq::{IteratorRandom, SliceRandom};
use serenity::async_trait;
use songbird::{
    events::{Event, EventContext, EventHandler as VoiceEventHandler, TrackEvent},
    input::{cached::Memory, File, Input},
    Songbird,
};

struct SongEndNotifier {
    m: std::sync::Arc<Songbird>,
    g: serenity::GuildId,
}

#[async_trait]
impl VoiceEventHandler for SongEndNotifier {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        let _ = self.m.remove(self.g).await;
        None
    }
}

///
/// Start a coin gamble
///
/// Enter `/coingamble <amount>`
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
    #[description = "Heads or Tails?"] choice: HeadsOrTail,
) -> Result<(), Error> {
    match minute_cooldown(ctx).await {
        Ok(_) => {}
        Err(e) => return Err(e),
    }
    let game_length = { ctx.data().config.read().unwrap().game_length_seconds };
    let db = &ctx.data().db;
    let game_starter = ctx.author().id.to_string();
    let user_balance = ctx.data().db.get_balance(ctx.author().id.get()).await?;
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
    db.subtract_balances(vec![game_starter.parse().unwrap()], amount)
        .await?;

    ctx.data().config.write().unwrap().bot_odds_game_counter += 1;

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
                "> ### {} HEADS OR TAILS?\n> **Bet {} {} **on the correct answer!\n> **Game Ends: **<t:{}:R>",
                JBUCK_EMOJI,
                amount,
                JBUCK_EMOJI,
                now + time_to_play as u64
            ))
            .components(components.clone())
    };

    let a = ctx.send(reply).await?;
    let id = a.message().await?.id;

    let mut coingame = CoinGame::new(
        ctx.author().id.get(),
        choice.clone(),
        amount,
        ctx.data().config.read().unwrap().side_chance,
        ctx.data().config.read().unwrap().bot_odds,
    );

    while let Some(mci) = serenity::ComponentInteractionCollector::new(ctx)
        .channel_id(ctx.channel_id())
        .custom_ids(vec!["Heads".to_string(), "Tails".to_string()])
        .message_id(id)
        .timeout(std::time::Duration::from_secs(
            (now + time_to_play as u64 - 1)
                - SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        ))
        .await
    {
        if ctx
            .data()
            .locked_balances
            .lock()
            .unwrap()
            .contains(&(mci.user.id.get()))
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
        match coingame
            .player_joined(&ctx.data().db, mci.user.id.get(), &mci.data.custom_id)
            .await
        {
            Ok(_) => {
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
            Err(GameError::PlayerCantAfford) => {
                let player_balance = db.get_balance(mci.user.id.get()).await?;
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
            Err(GameError::PlayerAlreadyJoined) => {
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

        let button2 = new_player_count_button(coingame.players.len() as i32);
        let button3 = new_pot_counter_button(coingame.pot);

        let mut msg = mci.message.clone();

        msg.edit(
            ctx,
            serenity::EditMessage::new().components(vec![serenity::CreateActionRow::Buttons(
                vec![new_heads_button(), new_tails_button(), button2, button3],
            )]),
        )
        .await?;
    }

    let reply = {
        let components = vec![serenity::CreateActionRow::Buttons(vec![
            new_heads_button().disabled(true),
            new_tails_button().disabled(true),
            new_player_count_button(coingame.players.len() as i32),
            new_pot_counter_button(coingame.pot),
        ])];
        CreateReply::default()
            .content(format!(
                "> ### {} HEADS OR TAILS?\n> **Bet {} {} **on the correct answer!\n> **Game is over!**",
                JBUCK_EMOJI,
                amount,
                JBUCK_EMOJI
            ))
            .components(components)
    };

    a.edit(ctx, reply).await?;

    let has_player = if let Some(p) = ctx.data().cursed_player {
        coingame.has_player(p)
    } else {
        false
    };

    if has_player {
        coingame.side_chance = 50;
    }
    let coin_flip_result = coingame
        .get_winner(&ctx.data().db, ctx.data().bot_id, ctx.data().crown_role_id)
        .await;

    let mut winners = vec![];

    let msg = match coin_flip_result.result {
        CoinSides::Side => match coin_flip_result.prize {
            0 => format!(
                "{} {}",
                get_landed_on_side_text(&mut ctx.data().rng.lock().unwrap(), has_player),
                get_troll_emoji(&mut ctx.data().rng.lock().unwrap())
            ),
            _ => {
                let detected = match has_player {
                    true => " *greg detected* ".to_owned(),
                    false => " ".to_owned(),
                };
                format!("###{}Woah, a side coin!\n No way to call a winner here {}\n+ {} {} added to today's lottery!",
                   detected, get_troll_emoji(&mut ctx.data().rng.lock().unwrap()), coin_flip_result.prize, JBUCK_EMOJI)
            }
        },
        _ => {
            let mut picked_heads_users = coingame
                .heads
                .iter()
                .map(|u| format!("<@{}>", u))
                .collect::<Vec<_>>()
                .join(" ");

            let mut picked_tails_users = coingame
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
            match coin_flip_result.result {
                CoinSides::Heads => {
                    winners = coingame.heads.clone();
                    picked_heads_users = format!(
                        "> {}\n> {} Congrats on {} {}!",
                        picked_heads_users, DOGE_PRAY_EMOJI, coin_flip_result.prize, JBUCK_EMOJI
                    );

                    if coin_flip_result.johnnys_multiplier.unwrap_or(0.0) > 1.0
                        && coin_flip_result.prize_with_multiplier - coin_flip_result.prize > 0
                    {
                        picked_heads_users = format!(
                            "{} +{} Bonus!",
                            picked_heads_users,
                            coin_flip_result.prize_with_multiplier - coin_flip_result.prize
                        );
                    }
                    picked_tails_users =
                        format!("> {}\n> {} So sad.", picked_tails_users, DOGE_CRY_EMOJI);
                }
                CoinSides::Tails => {
                    winners = coingame.tails.clone();
                    picked_heads_users =
                        format!("> {}\n> {} So sad.", picked_heads_users, DOGE_CRY_EMOJI);
                    picked_tails_users = format!(
                        "> {}\n> {} Congrats on {} {}!",
                        picked_tails_users, DOGE_PRAY_EMOJI, coin_flip_result.prize, JBUCK_EMOJI
                    );
                    if coin_flip_result.johnnys_multiplier.unwrap_or(0.0) > 1.0
                        && coin_flip_result.prize_with_multiplier - coin_flip_result.prize > 0
                    {
                        picked_tails_users = format!(
                            "{} +{} Bonus!",
                            picked_tails_users,
                            coin_flip_result.prize_with_multiplier - coin_flip_result.prize
                        );
                    }
                }
                _ => {}
            };

            let mut a = format!(
                "> ### {} IT WAS {}!\n> \n",
                JBUCK_EMOJI,
                coin_flip_result.result.to_uppercase()
            );
            a.push_str(&format!("> **Picked Heads**\n{}\n> ", picked_heads_users));

            a.push_str(&format!("\n> **Picked Tails**\n{}\n", picked_tails_users));

            if coin_flip_result.remainder.unwrap_or(0) > 0 {
                a.push_str(&format!(
                    "> \n> +{} {} to <@{}> ||(Crown's Tax)||",
                    coin_flip_result.remainder.unwrap(),
                    JBUCK_EMOJI,
                    coin_flip_result.leader.unwrap()
                ));
            }
            a
        }
    };

    let message = {
        CreateReply::default()
            .content(msg)
            .allowed_mentions(serenity::CreateAllowedMentions::new().empty_users())
    };
    ctx.send(message).await?;

    if winners.is_empty() {
        tracing::info!("no winners");
        return Ok(());
    }

    if let Some(winner) = winners.first() {
        if winner == &ctx.data().bot_id {
            tracing::info!("bot won");
            return Ok(());
        }
    }

    let threshold = match ctx.data().config.read() {
        Ok(config) => Some(config.voice_channel_celebration_amount),
        Err(_) => None,
    };

    if let Some(threshold) = threshold {
        if coin_flip_result.prize > threshold {
            play_obnoxious_celebration(ctx).await;
        }
    }

    Ok(())
}

async fn play_obnoxious_celebration(ctx: Context<'_>) {
    let (guild_id, channel_id) = {
        let guild = ctx.guild().unwrap();
        let voice_states = guild.voice_states.clone();
        if voice_states.is_empty() {
            tracing::warn!("Nobody in a voice channel, skipping obnoxious stuff");
            return;
        }
        let channel_id = voice_states
            .values()
            .choose(&mut rand::thread_rng())
            .unwrap()
            .channel_id;
        (guild.id, channel_id)
    };

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            tracing::info!("no channel");
            return;
        }
    };

    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird init")
        .clone();
    let paths = match std::fs::read_dir("./resources/") {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("Error searching for sounds to play: {}", e);
            return;
        }
    };

    let path: PathBuf;
    if let Some(p) = paths.choose(&mut rand::thread_rng()) {
        path = p.unwrap().path();
    } else {
        tracing::warn!("could not find sounds in directory");
        return;
    }

    if let Ok(handler_lock) = manager.join(guild_id, connect_to).await {
        let mut handler = handler_lock.lock().await;
        handler.add_global_event(TrackEvent::Error.into(), TrackErrorNotifier);
        let audio_src = Memory::new(File::new(path).into()).await.unwrap();

        let _ = audio_src.raw.spawn_loader();
        let source: Input = audio_src.new_handle().into();

        let h = handler.play_input(source);

        let _ = h.add_event(
            Event::Track(TrackEvent::End),
            SongEndNotifier {
                m: manager.clone(),
                g: guild_id,
            },
        );
    }
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
fn get_landed_on_side_text(a: &mut rand::rngs::StdRng, has_player: bool) -> String {
    if has_player {
        return "*greg detected* ".to_owned() + LANDEDSIDE.choose(a).unwrap().to_string().as_str();
    }
    LANDEDSIDE.choose(a).unwrap().to_string()
}

pub fn get_troll_emoji(a: &mut rand::rngs::StdRng) -> String {
    let emoji = [DOGE_TROLL_EMOJI_1, DOGE_TROLL_EMOJI_2]
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
            user: Some(time::Duration::from_secs(30)),
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
        return Err("You can use this command again in 30 seconds".into());
    }
    Ok(())
}
struct TrackErrorNotifier;

#[async_trait]
impl VoiceEventHandler for TrackErrorNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(track_list) = ctx {
            for (state, handle) in *track_list {
                println!(
                    "Track {:?} encountered an error: {:?}",
                    handle.uuid(),
                    state.playing
                );
            }
        }

        None
    }
}
