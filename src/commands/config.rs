use poise::CreateReply;

use crate::{database::ConfigDatabase, Context, Error};

#[derive(Debug, poise::ChoiceParameter, Clone)]
pub enum ConfigOption {
    DailyLimit,
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
        ConfigOption::DailyLimit => {
            let limit = value
                .parse::<i32>()
                .map_err(|_| Error::from("Invalid value".to_string()))?;
            ctx.data()
                .db
                .set_config_value("daily_upper_limit", value.as_str())
                .await
                .unwrap();
            ctx.data().config.write().unwrap().daily_upper_limit = limit;
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
    let response = format!(
        "Daily upper limit: {}\n",
        config.daily_upper_limit.unwrap_or(0),
    );
    let reply = CreateReply::default().content(response).ephemeral(true);
    ctx.send(reply).await?;
    Ok(())
}
