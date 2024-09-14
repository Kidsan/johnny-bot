use poise::CreateReply;

use crate::{
    database::{self, ConfigDatabase},
    Context, Error,
};

#[derive(Debug, poise::ChoiceParameter, Clone)]
pub enum ConfigOption {
    DailyLimit,
    BotOdds,
    BotOddsGameLimit,
    GameLengthSeconds,
    RobberyLengthSeconds,
    LotteryTicketPrice,
    LotteryBasePrize,
    FutureLotteryTicketPrice,
    FutureLotteryBasePrize,
    SideChance,
    CommunityEmojiPrice,
    BonesPriceMinFluctuation,
    BonesPriceMaxFluctuation,
    ForceBonesPriceUpdate,
    LotteryWinner,
    ForceEgg,
}

///
/// manage configuration
///
#[poise::command(
    slash_command,
    category = "Admin",
    hide_in_help,
    default_member_permissions = "ADMINISTRATOR",
    subcommands("set", "get")
)]
pub async fn config(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

///
/// set configuration
///
#[poise::command(
    slash_command,
    category = "Admin",
    hide_in_help,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn set(ctx: Context<'_>, option: ConfigOption, value: String) -> Result<(), Error> {
    match option {
        ConfigOption::GameLengthSeconds => {
            let length = parse_value::<i32>(&value)?;
            ctx.data()
                .db
                .set_config_value(database::ConfigKey::GameLengthSeconds, value.as_str())
                .await
                .unwrap();
            ctx.data().config.write().unwrap().game_length_seconds = length;
        }
        ConfigOption::DailyLimit => {
            let limit = parse_value::<i32>(&value)?;
            ctx.data()
                .db
                .set_config_value(database::ConfigKey::DailyUpperLimit, value.as_str())
                .await
                .unwrap();
            ctx.data().config.write().unwrap().daily_upper_limit = limit;
        }
        ConfigOption::BotOdds => {
            let odds = parse_value::<f32>(&value)?;
            if !(0.0..=1.0).contains(&odds) {
                return Err(Error::from("Invalid value".to_string()));
            }
            ctx.data()
                .db
                .set_config_value(database::ConfigKey::BotOdds, value.as_str())
                .await
                .unwrap();
            ctx.data()
                .db
                .set_config_value(
                    database::ConfigKey::BotOddsUpdated,
                    &chrono::Utc::now().timestamp().to_string(),
                )
                .await
                .unwrap();
            ctx.data().config.write().unwrap().bot_odds = odds;
        }
        ConfigOption::LotteryTicketPrice => {
            let price = parse_value::<i32>(&value)?;
            ctx.data()
                .db
                .set_config_value(database::ConfigKey::LotteryTicketPrice, value.as_str())
                .await
                .unwrap();
            ctx.data().config.write().unwrap().lottery_ticket_price = price;
        }
        ConfigOption::LotteryBasePrize => {
            let prize = parse_value::<i32>(&value)?;
            ctx.data()
                .db
                .set_config_value(database::ConfigKey::LotteryBasePrize, value.as_str())
                .await
                .unwrap();
            ctx.data().config.write().unwrap().lottery_base_prize = prize;
        }
        ConfigOption::FutureLotteryTicketPrice => {
            let price = parse_value::<i32>(&value)?;
            ctx.data()
                .db
                .set_config_value(
                    database::ConfigKey::FutureLotteryTicketPrice,
                    value.as_str(),
                )
                .await
                .unwrap();
            ctx.data()
                .config
                .write()
                .unwrap()
                .future_lottery_ticket_price = price;
        }
        ConfigOption::FutureLotteryBasePrize => {
            let prize = parse_value::<i32>(&value)?;
            ctx.data()
                .db
                .set_config_value(database::ConfigKey::FutureLotteryBasePrize, value.as_str())
                .await
                .unwrap();
            ctx.data().config.write().unwrap().future_lottery_base_prize = prize;
        }
        ConfigOption::SideChance => {
            let chance = parse_value::<u32>(&value)?;
            if !(0..=100).contains(&chance) {
                return Err(Error::from("Chance must be in range 0..=100".to_string()));
            }
            ctx.data()
                .db
                .set_config_value(database::ConfigKey::SideChance, value.as_str())
                .await
                .unwrap();
            ctx.data().config.write().unwrap().side_chance = chance;
        }
        ConfigOption::CommunityEmojiPrice => {
            let price = parse_value::<i32>(&value)?;
            ctx.data()
                .db
                .set_config_value(database::ConfigKey::CommunityEmojiPrice, value.as_str())
                .await
                .unwrap();
            ctx.data().config.write().unwrap().community_emoji_price = price;
        }
        ConfigOption::BonesPriceMinFluctuation => {
            let price = parse_value::<i32>(&value)?;
            ctx.data()
                .db
                .set_config_value(database::ConfigKey::BonesPriceMin, value.as_str())
                .await
                .unwrap();
            ctx.data().config.write().unwrap().bones_price_min = price;
        }
        ConfigOption::BonesPriceMaxFluctuation => {
            let price = parse_value::<i32>(&value)?;
            ctx.data()
                .db
                .set_config_value(database::ConfigKey::BonesPriceMax, value.as_str())
                .await
                .unwrap();
            ctx.data().config.write().unwrap().bones_price_max = price;
        }
        ConfigOption::ForceBonesPriceUpdate => {
            let force = parse_value::<bool>(&value)?;
            ctx.data()
                .db
                .set_config_value(database::ConfigKey::ForceBonesPriceUpdate, value.as_str())
                .await
                .unwrap();
            ctx.data().config.write().unwrap().bones_price_force_update = force;
        }
        ConfigOption::BotOddsGameLimit => {
            let limit = parse_value::<u8>(&value)?;
            ctx.data()
                .db
                .set_config_value(database::ConfigKey::BotOddsGameLimit, value.as_str())
                .await
                .unwrap();
            ctx.data().config.write().unwrap().bot_odds_game_limit = limit;
        }
        ConfigOption::LotteryWinner => {
            let winner = value
                .parse::<u64>()
                .map_err(|_| Error::from("Invalid value".to_string()))?;
            ctx.data()
                .db
                .set_config_value(database::ConfigKey::LotteryWinner, value.as_str())
                .await
                .unwrap();
            ctx.data().config.write().unwrap().lottery_winner = Some(winner);
        }
        ConfigOption::ForceEgg => {
            tracing::info!("force egg");
            let force = parse_value::<bool>(&value)?;
            ctx.data()
                .db
                .set_config_value(database::ConfigKey::ForceEgg, value.as_str())
                .await
                .unwrap();
            ctx.data().config.write().unwrap().force_egg = force;
        }
        ConfigOption::RobberyLengthSeconds => {
            let length = parse_value::<i8>(&value)?;
            ctx.data()
                .db
                .set_config_value(database::ConfigKey::RobberyLengthSeconds, value.as_str())
                .await
                .unwrap();
            ctx.data().config.write().unwrap().robbery_length_seconds = length;
        }
    }
    let reply = CreateReply::default().content("Success").ephemeral(true);
    ctx.send(reply).await?;
    Ok(())
}

fn parse_value<T: std::str::FromStr>(value: &str) -> Result<T, Error> {
    value
        .parse::<T>()
        .map_err(|_| Error::from("Invalid value".to_string()))
}

///
/// get configuration
///
#[poise::command(
    slash_command,
    category = "Admin",
    hide_in_help,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn get(ctx: Context<'_>) -> Result<(), Error> {
    let config = ctx.data().db.get_config().await.unwrap();
    let response = format!("{config}");
    let reply = CreateReply::default().content(response).ephemeral(true);
    ctx.send(reply).await?;
    Ok(())
}
