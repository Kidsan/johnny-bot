use crate::database::{ChannelDatabase, RoleDatabase, ShopDatabase};
mod commands;
mod database;
mod discord;
mod eventhandler;
mod game;
mod johnny;
mod texts;

use database::ConfigDatabase;
use poise::{serenity_prelude as serenity, CreateReply};
use std::sync::mpsc;
use std::sync::RwLock;
use std::{
    collections::{HashMap, HashSet},
    env::var,
    sync::{Arc, Mutex},
    time::Duration,
};

// Types used by all command functions
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

type RolePrice = (i32, Option<serenity::RoleId>);

#[derive(Debug)]
pub struct Config {
    daily_upper_limit: i32,
    bot_odds: f32,
    bot_odds_updated: Option<chrono::DateTime<chrono::Utc>>,
    bot_odds_game_limit: u8,
    bot_odds_game_counter: u8,
    game_length_seconds: i32,
    robbery_length_seconds: i8,
    lottery_ticket_price: i32,
    lottery_base_prize: i32,
    future_lottery_ticket_price: i32,
    future_lottery_base_prize: i32,
    side_chance: u32,
    community_emoji_price: i32,
    bones_price: i32,
    bones_price_updated: chrono::DateTime<chrono::Utc>,
    bones_price_min: i32,
    bones_price_max: i32,
    bones_price_last_was_increase: Option<bool>,
    bones_price_force_update: bool,
    lottery_winner: Option<u64>,
    force_egg: bool,
    just_egged: Option<u64>,
}

impl Config {
    fn from(input: database::Config) -> Self {
        Self {
            daily_upper_limit: input.daily_upper_limit.unwrap_or(0),
            bot_odds: input.bot_odds.unwrap_or(0.5),
            game_length_seconds: input.game_length_seconds.unwrap_or(30),
            robbery_length_seconds: input.robbery_length_seconds.unwrap_or(60),
            lottery_ticket_price: input.lottery_ticket_price.unwrap_or(5),
            lottery_base_prize: input.lottery_base_prize.unwrap_or(10),
            future_lottery_ticket_price: input.future_lottery_ticket_price.unwrap_or(5),
            future_lottery_base_prize: input.future_lottery_base_prize.unwrap_or(10),
            side_chance: input.side_chance.unwrap_or(2),
            community_emoji_price: input.community_emoji_price,
            bones_price: input.bones_price,
            bones_price_updated: input.bones_price_updated,
            bones_price_min: input.bones_price_min,
            bones_price_max: input.bones_price_max,
            bones_price_last_was_increase: input.bones_price_last_was_increase,
            bones_price_force_update: input.force_bones_price_update.unwrap_or(false),
            bot_odds_updated: input.bot_odds_updated,
            bot_odds_game_limit: input.bot_odds_game_limit.unwrap_or(10),
            bot_odds_game_counter: input.bot_odds_game_counter.unwrap_or(0),
            lottery_winner: input.lottery_winner,
            force_egg: input.force_egg,
            just_egged: None,
        }
    }
}

// Custom user data passed to all command functions
#[derive(Debug)]
pub struct Data {
    games: Mutex<HashMap<String, game::Game>>,
    db: database::Database,
    rng: Mutex<rand::rngs::StdRng>,
    locked_balances: Mutex<HashSet<u64>>,
    bot_id: u64,
    blackjack_active: Mutex<bool>,
    paid_channels: Mutex<HashMap<serenity::ChannelId, i32>>,
    roles: Arc<RwLock<HashMap<serenity::RoleId, RolePrice>>>,
    unique_roles: Mutex<HashSet<serenity::RoleId>>,
    crown_role_id: u64,
    active_checks: Mutex<HashSet<u64>>,
    config: Arc<RwLock<Config>>,
}

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    // This is our custom error handler
    // They are many errors that can occur, so we only handle the ones we want to customize
    // and forward the rest to the default handler
    match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {:?}", error),
        poise::FrameworkError::Command { error, ctx, .. } => {
            tracing::error!("Error in command `{}`: {:?}", ctx.command().name, error,);
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                tracing::error!("Error while handling error: {}", e)
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let bot_id = match var("BOT_ID") {
        Ok(id) => id.parse().unwrap(),
        Err(_) => 1049354446578143252,
    };

    let crown_role_id = match var("CROWN_ROLE_ID") {
        Ok(id) => id.parse().unwrap(),
        Err(_) => "1237724109756956753".to_string().parse().unwrap(),
    };

    let den_channel_id = match var("DEN_CHANNEL_ID") {
        Ok(id) => poise::serenity_prelude::ChannelId::new(id.parse().unwrap()),
        Err(_) => poise::serenity_prelude::ChannelId::new(1049354446578143252),
    };
    let in_dev = var("DEV_SETTINGS").is_ok();

    tracing_subscriber::fmt().init();

    let mut commands = vec![
        commands::help::help(),
        commands::say::say(),
        commands::checkbucks::checkbucks(),
        commands::balance::balance(),
        commands::register::register(),
        commands::give::give(),
        commands::fine::fine(),
        commands::addbucks::add_bucks(),
        commands::removebucks::remove_bucks(),
        commands::transfer::transfer(),
        commands::award::award(),
        commands::coingamble::coingamble(),
        commands::daily::daily(),
        commands::stats::stats(),
        commands::burn::bury(),
        commands::robbingevent::robbingevent(),
        commands::leaderboard::leaderboard(),
        commands::rockpaperscissors::rpsgamble(),
        commands::paidchannels::setchannelprice(),
        commands::buy::buy(),
        commands::buy::shop(),
        commands::buy::setroleprice(),
        commands::buy::decay(),
        commands::buy::list_decays(),
        commands::buy::list_prices(),
        commands::leaderboard::crownleaderboard(),
        commands::config::config(),
        commands::lottery::lottery(),
        commands::giveaway::giveaway(),
        commands::buy::bones_status(),
        commands::buy::sell(),
        commands::gamble::gamble(),
        commands::report::report(),
        commands::report::reports(),
        commands::report::deletereport(),
        commands::robbingevent::buyrobbery(),
    ];

    if var("MOUNT_ALL").is_ok() {
        tracing::debug!("Mounting all commands");
        commands.push(commands::blackjack::blackjack());
    };

    let db: database::Database = database::Database::new().await.unwrap();
    let db2 = database::Database::new().await.unwrap();

    setup_community_emojis(&db).await;

    let paid_channels = db.get_paid_channels().await.unwrap();
    let paid_channels_map: HashMap<_, _> = paid_channels
        .iter()
        .map(|(channel_id, amount)| (serenity::ChannelId::new(*channel_id), *amount))
        .collect();

    let paid_roles = db.get_purchasable_roles().await.unwrap();
    let roles = paid_roles
        .iter()
        .map(|role| {
            let required_role = role.required_role_id;
            (
                serenity::RoleId::new(role.role_id),
                (role.price, required_role.map(serenity::RoleId::new)),
            )
        })
        .collect::<HashMap<_, _>>();

    let rc = Arc::new(RwLock::new(roles.clone()));
    let rc_clone = Arc::clone(&rc);
    let c = db.get_config().await.unwrap();
    let config = Arc::new(RwLock::new(Config::from(c)));
    let config_clone = Arc::clone(&config);

    let unique_roles = paid_roles
        .iter()
        .filter(|role| role.only_one)
        .map(|role| serenity::RoleId::new(role.role_id))
        .collect::<HashSet<_>>();

    // FrameworkOptions contains all of poise's configuration option in one struct
    // Every option can be omitted to use its default value
    let options = poise::FrameworkOptions {
        commands,
        manual_cooldowns: true,
        // The global error handler for all error cases that may occur
        on_error: |error| Box::pin(on_error(error)),
        // This code is run before every command
        pre_command: |ctx| {
            Box::pin(async move {
                tracing::debug!("Executing command {}...", ctx.command().qualified_name);
            })
        },
        // This code is run after a command if it was successful (returned Ok)
        post_command: |ctx| {
            Box::pin(async move {
                tracing::debug!(
                    "Executed command {} in {}!",
                    ctx.command().qualified_name,
                    ctx.channel_id()
                );
            })
        },
        // Every command invocation must pass this check to continue execution
        command_check: Some(|ctx| {
            Box::pin(async move {
                if ctx.author().id == 123456789 {
                    return Ok(false);
                }

                if ctx.command().name.as_str() == "leaderboard"
                    && !ctx.data().locked_balances.lock().unwrap().is_empty()
                {
                    let reply = {
                        CreateReply::default()
                            .content("<:dogeTroll:1160530414490886264>")
                            .ephemeral(true)
                    };
                    ctx.send(reply).await?;
                    return Ok(false);
                }

                if [
                    "give",
                    "coingamble",
                    "bury",
                    "buyrobbery",
                    "rpsgamble",
                    "buy",
                    "role", // subcommand of buy role but its seen as just "role"
                    "bones",
                    "emoji",
                    "lottery",
                    "daily",
                ]
                .contains(&ctx.command().name.as_str())
                    && ctx
                        .data()
                        .locked_balances
                        .lock()
                        .unwrap()
                        .contains(&ctx.author().id.get())
                {
                    let reply = {
                        CreateReply::default()
                            .content("Nice try, but you can't do that right now. Try again after.")
                            .ephemeral(true)
                    };
                    ctx.send(reply).await?;
                    return Ok(false);
                }

                Ok(true)
            })
        }),
        // Enforce command checks even for owners (enforced by default)
        // Set to true to bypass checks, which is useful for testing
        skip_checks_for_owners: false,
        event_handler: |ctx, event, _framework, data| {
            Box::pin(eventhandler::event_handler(ctx, event, _framework, data))
        },
        ..Default::default()
    };

    let framework = poise::Framework::builder()
        .setup(move |_ctx, _ready, _framework| {
            Box::pin(async move {
                tracing::info!("Logged in as {}", _ready.user.name);
                Ok(Data {
                    games: Mutex::new(HashMap::new()),
                    db,
                    rng: Mutex::new(rand::SeedableRng::from_entropy()),
                    locked_balances: Mutex::new(HashSet::new()),
                    bot_id,
                    blackjack_active: Mutex::new(false),
                    paid_channels: Mutex::new(paid_channels_map),
                    roles: rc,
                    unique_roles: Mutex::new(unique_roles),
                    crown_role_id,
                    active_checks: Mutex::new(HashSet::new()),
                    config,
                })
            })
        })
        .options(options)
        .build();

    let token = var("DISCORD_TOKEN")
        .expect("Missing `DISCORD_TOKEN` env var, see README for more information.");
    let intents = serenity::GatewayIntents::non_privileged()
        | serenity::GatewayIntents::MESSAGE_CONTENT
        | serenity::GatewayIntents::GUILD_MEMBERS;

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;

    let (tx, rx) = mpsc::channel();
    let johnny = johnny::Johnny::new(
        db2,
        rc_clone,
        config_clone,
        den_channel_id,
        client.as_ref().unwrap(),
        in_dev,
    );
    tokio::spawn(async move {
        johnny.start(rx).await;
    });

    let shard_manager = client.as_ref().unwrap().shard_manager.clone();

    tokio::spawn(async move {
        client.unwrap().start().await.unwrap();
    });
    wait_until_shutdown().await;
    let _ = tx.send(());
    shard_manager.shutdown_all().await;
}

#[cfg(unix)]
async fn wait_until_shutdown() {
    use tokio::signal::unix::{signal, SignalKind};

    let mut sigint = signal(SignalKind::interrupt()).unwrap();
    let mut sighup = signal(SignalKind::hangup()).unwrap();
    let mut sigterm = signal(SignalKind::terminate()).unwrap();
    tokio::select! {
        v = sigint.recv() => {
            tracing::debug!("Received A SIGINT, shutting down...");
            v.unwrap()
        },
        v = sigterm.recv() => {
            tracing::debug!("Received SIGTERM, shutting down...");
            v.unwrap()
        }
        v = sighup.recv() => {
            tracing::debug!("Received SIGHUP, shutting down...");
            v.unwrap()
        }
    }
}

#[cfg(windows)]
async fn wait_until_shutdown() {
    use tokio::signal::windows::{signal, SignalKind};
    tokio::signal::ctrl_c().await.unwrap();
    tracing::debug!("Received CTRL-C, shutting down...");
}

async fn setup_community_emojis(db: &database::Database) {
    let emojis = match db.get_community_emojis().await {
        Ok(emojis) => emojis,
        Err(_) => {
            tracing::warn!("Failed to setup community emojis");
            return;
        }
    };

    let emoji_names = ["neds1", "neds2", "neds3", "neds4", "neds5"];
    let mut missing = vec![];

    for e in emoji_names {
        if !emojis.iter().any(|emoji| emoji.name == e) {
            missing.push(e);
        }
    }

    for missing in missing {
        match db.add_community_emoji(missing).await {
            Ok(_) => tracing::info!("Added missing community emoji: {}", missing),
            Err(e) => tracing::error!("Failed to add missing community emoji: {}", e),
        }
    }
}
