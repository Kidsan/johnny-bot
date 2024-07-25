use crate::database::ChannelDatabase;
use crate::database::RoleDatabase;
mod commands;
mod database;
mod eventhandler;
mod game;
mod johnny;
mod texts;

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
    game_length_seconds: i32,
    lottery_ticket_price: i32,
    lottery_base_prize: i32,
    future_lottery_ticket_price: i32,
    future_lottery_base_prize: i32,
}

// Custom user data passed to all command functions
#[derive(Debug)]
pub struct Data {
    games: Mutex<HashMap<String, game::Game>>,
    db: database::Database,
    side_chance: i32,
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
            println!("Error in command `{}`: {:?}", ctx.command().name, error,);
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                println!("Error while handling error: {}", e)
            }
        }
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let side_chance = match var("SIDE_CHANCE") {
        Ok(chance) => chance.parse::<i32>().unwrap(),
        Err(_) => 2,
    };

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

    let mut commands = vec![
        commands::help::help(),
        commands::say::say(),
        commands::checkbucks::checkbucks(),
        commands::balance::balance(),
        commands::register::register(),
        commands::gamble::gamble(),
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
        commands::robbingevent::buyrobbery(),
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
    ];

    if var("MOUNT_ALL").is_ok() {
        println!("Mounting all commands");
        commands.push(commands::blackjack::blackjack());
    };

    let db: database::Database = database::Database::new().await.unwrap();
    let db2 = database::Database::new().await.unwrap();

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
    let config = Arc::new(RwLock::new(Config {
        daily_upper_limit: 0,
        bot_odds: 0.5,
        game_length_seconds: 30,
        lottery_ticket_price: 5,
        lottery_base_prize: 10,
        future_lottery_ticket_price: 5,
        future_lottery_base_prize: 10,
    }));
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
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some("~".into()),
            edit_tracker: Some(Arc::new(poise::EditTracker::for_timespan(
                Duration::from_secs(3600),
            ))),
            additional_prefixes: vec![
                poise::Prefix::Literal("hey bot"),
                poise::Prefix::Literal("hey bot,"),
            ],
            ..Default::default()
        },
        // The global error handler for all error cases that may occur
        on_error: |error| Box::pin(on_error(error)),
        // This code is run before every command
        pre_command: |ctx| {
            Box::pin(async move {
                println!("Executing command {}...", ctx.command().qualified_name);
            })
        },
        // This code is run after a command if it was successful (returned Ok)
        post_command: |ctx| {
            Box::pin(async move {
                println!(
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
                println!("Logged in as {}", _ready.user.name);
                Ok(Data {
                    games: Mutex::new(HashMap::new()),
                    db,
                    side_chance,
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
    let intents =
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;

    let sender = client.as_ref().unwrap().http.clone();
    let (tx, rx) = mpsc::channel();
    let johnny = johnny::Johnny::new(
        db2,
        rc_clone,
        config_clone,
        den_channel_id,
        Some(sender),
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
            println!("Received A SIGINT, shutting down...");
            v.unwrap()
        },
        v = sigterm.recv() => {
            println!("Received SIGTERM, shutting down...");
            v.unwrap()
        }
        v = sighup.recv() => {
            println!("Received SIGHUP, shutting down...");
            v.unwrap()
        }
    }
}

#[cfg(windows)]
async fn wait_until_shutdown() {
    use tokio::signal::windows::{signal, SignalKind};
    tokio::signal::ctrl_c().await.unwrap();
    println!("Received CTRL-C, shutting down...");
}
