use poise::CreateReply;

use crate::{
    database::{self, ConfigDatabase},
    Context, Error,
};

#[derive(Debug, poise::ChoiceParameter, Clone)]
pub enum ConfigOption {
    DailyLimit,
    BotOdds,
    GameLengthSeconds,
    LotteryTicketPrice,
    LotteryBasePrize,
    FutureLotteryTicketPrice,
    FutureLotteryBasePrize,
    SideChance,
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
            let length = value
                .parse::<i32>()
                .map_err(|_| Error::from("Invalid value".to_string()))?;
            ctx.data()
                .db
                .set_config_value(database::ConfigKey::GameLengthSeconds, value.as_str())
                .await
                .unwrap();
            ctx.data().config.write().unwrap().game_length_seconds = length;
        }
        ConfigOption::DailyLimit => {
            let limit = value
                .parse::<i32>()
                .map_err(|_| Error::from("Invalid value".to_string()))?;
            ctx.data()
                .db
                .set_config_value(database::ConfigKey::DailyUpperLimit, value.as_str())
                .await
                .unwrap();
            ctx.data().config.write().unwrap().daily_upper_limit = limit;
        }
        ConfigOption::BotOdds => {
            let odds = value
                .parse::<f32>()
                .map_err(|_| Error::from("Invalid value".to_string()))?;
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
            let price = value
                .parse::<i32>()
                .map_err(|_| Error::from("Invalid value".to_string()))?;
            ctx.data()
                .db
                .set_config_value(database::ConfigKey::LotteryTicketPrice, value.as_str())
                .await
                .unwrap();
            ctx.data().config.write().unwrap().lottery_ticket_price = price;
        }
        ConfigOption::LotteryBasePrize => {
            let prize = value
                .parse::<i32>()
                .map_err(|_| Error::from("Invalid value".to_string()))?;
            ctx.data()
                .db
                .set_config_value(database::ConfigKey::LotteryBasePrize, value.as_str())
                .await
                .unwrap();
            ctx.data().config.write().unwrap().lottery_base_prize = prize;
        }
        ConfigOption::FutureLotteryTicketPrice => {
            let price = value
                .parse::<i32>()
                .map_err(|_| Error::from("Invalid value".to_string()))?;
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
            let prize = value
                .parse::<i32>()
                .map_err(|_| Error::from("Invalid value".to_string()))?;
            ctx.data()
                .db
                .set_config_value(database::ConfigKey::FutureLotteryBasePrize, value.as_str())
                .await
                .unwrap();
            ctx.data().config.write().unwrap().future_lottery_base_prize = prize;
        }
        ConfigOption::SideChance => {
            let chance = value
                .parse::<i32>()
                .map_err(|_| Error::from("Invalid value".to_string()))?;
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
    }
    let reply = CreateReply::default().content("Success").ephemeral(true);
    ctx.send(reply).await?;
    Ok(())
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
