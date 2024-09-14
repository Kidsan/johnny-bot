use crate::{
    commands::{
        lottery::buylotteryticket,
        robbingevent::{buyrobbery, get_discord_name},
    },
    database::{BalanceDatabase, RoleDatabase, ShopDatabase},
    discord::JBUCK_EMOJI,
    johnny::is_weekend,
    Context, Error,
};
use base64::{engine::general_purpose, Engine as _};
use chrono::{Datelike, Days, NaiveTime};
use poise::CreateReply;
use serenity::all::Emoji;

///
/// List the items for sale in the shop
///
/// Enter `/shop`
/// ```
/// /shop
/// ```
#[poise::command(slash_command)]
pub async fn shop(ctx: Context<'_>) -> Result<(), Error> {
    let crown_holder = {
        ctx.data()
            .db
            .get_unique_role_holder(ctx.data().crown_role_id)
            .await?
    };
    let reply = {
        let roles = { ctx.data().roles.read().unwrap().clone() };
        let mut a = ctx
            .serenity_context()
            .http
            .get_guild_roles(ctx.guild_id().unwrap())
            .await?
            .iter()
            .filter_map(|r| {
                if roles.contains_key(&r.id) {
                    Some((r.id, r.position))
                } else {
                    None
                }
            })
            .collect::<Vec<(serenity::model::id::RoleId, u16)>>();
        a.sort_by_key(|r| r.1);
        a.reverse();
        let uniques = ctx.data().unique_roles.lock().unwrap();
        let role_prices = a
            .iter()
            .map(|(role_id, _)| {
                format!(
                    "> <@&{}> - {} {}{}{}",
                    role_id,
                    roles.get(role_id).unwrap().0,
                    JBUCK_EMOJI,
                    if uniques.contains(role_id) {
                        if role_id.get() == ctx.data().crown_role_id {
                            if let Some(crown_holder) = &crown_holder {
                                format!(" (Unique - Current holder: <@{}>)", crown_holder.user_id)
                            } else {
                                " (Unique)".to_string()
                            }
                        } else {
                            " (Unique)".to_string()
                        }
                    } else {
                        "".to_string()
                    },
                    if roles.get(role_id).unwrap().1.is_some() {
                        format!(" (Requires <@&{}>)", roles.get(role_id).unwrap().1.unwrap())
                    } else {
                        "".to_string()
                    }
                )
            })
            .collect::<Vec<String>>()
            .join("\n");
        let formatted_role_prices = format!("**Roles for sale:**\n{}", role_prices);
        let formatted_emoji_prices = format!("**Emoji:**\n> Community Emoji: {} {}\n> *The oldest of the community emojis gets replaced*\n\n", ctx.data().config.read().unwrap().community_emoji_price, JBUCK_EMOJI);
        let formatted_bones_prices = format!(
            "**Bones:**\n> Bones: {} {}\n\n",
            ctx.data().config.read().unwrap().bones_price,
            JBUCK_EMOJI
        );
        let header = format!("### {} Shop {} ###\n\n", JBUCK_EMOJI, JBUCK_EMOJI);
        let footer = String::from("\n\nMore info on roles at: https://canary.discord.com/channels/1128350000343167130/1227274968312844320\nTo buy a role use the **/buy role** command.");

        CreateReply::default().content(format!("{header}{formatted_bones_prices}{formatted_emoji_prices}{formatted_role_prices}{footer}")).ephemeral(true)
    };
    ctx.send(reply).await?;

    Ok(())
}

///
/// Set the price for a role
///
/// Enter `/setroleprice [role] [price] [increment] [required_role]`
/// ```
/// /setroleprice @Johnny'sChosen 5 1
/// ```
#[poise::command(
    slash_command,
    category = "Admin",
    default_member_permissions = "ADMINISTRATOR",
    hide_in_help
)]
pub async fn setroleprice(
    ctx: Context<'_>,
    #[description = "The role to set the price for"] role: poise::serenity_prelude::Role,
    #[min = 0]
    #[description = "The price for this role"]
    price: i32,
    #[min = 0]
    #[description = "The amount to increase the price buy after a purchase"]
    increment: Option<i32>,
    #[description = "An optional prerequisite role"] required_role: Option<
        poise::serenity_prelude::Role,
    >,
    #[description = "Can only one person have this role?"] only_one: Option<bool>,
) -> Result<(), Error> {
    let required_role_id = required_role
        .clone()
        .map(|role| role.id.to_string().parse().unwrap());
    ctx.data()
        .db
        .set_role_price(
            role.id.to_string().parse()?,
            price,
            increment,
            required_role_id,
            only_one,
        )
        .await?;

    let id = match required_role {
        Some(role) => Some(role.id),
        None => None,
    };
    ctx.data()
        .roles
        .write()
        .unwrap()
        .insert(role.id, (price, id));

    match only_one {
        Some(true) => {
            ctx.data().unique_roles.lock().unwrap().insert(role.id);
        }
        Some(false) => {
            ctx.data().unique_roles.lock().unwrap().remove(&role.id);
        }
        None => {
            ctx.data().unique_roles.lock().unwrap().remove(&role.id);
        }
    }

    if price == 0 {
        ctx.data().roles.write().unwrap().remove(&role.id);
        let reply = {
            CreateReply::default()
                .content(format!("You have removed the role {} from the shop!", role))
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Ok(());
    }
    let reply = {
        CreateReply::default()
            .content(format!(
                "You have set the price for the role {} to {}!",
                role, price
            ))
            .ephemeral(true)
    };
    ctx.send(reply).await?;
    Ok(())
}

pub async fn incrementroleprice(ctx: Context<'_>, role_id: String) -> Result<(), Error> {
    ctx.data().db.increment_role_price(role_id).await?;
    let prices = ctx.data().db.get_purchasable_roles().await?;
    {
        let mut roles = ctx.data().roles.write().unwrap();
        for price in prices {
            roles.insert(
                poise::serenity_prelude::RoleId::new(price.role_id),
                (
                    price.price,
                    price
                        .required_role_id
                        .map(poise::serenity_prelude::RoleId::new),
                ),
            );
        }
    }
    Ok(())
}

///
/// Buy something with your JBucks
///
/// Enter `/buy `
/// ```
/// /buy role @role
/// ```
#[poise::command(
    slash_command,
    subcommands("role", "emoji", "bones", "buylotteryticket"),
    subcommand_required
)]
pub async fn buy(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

pub async fn complete_roles<'a>(
    ctx: Context<'a>,
    _partial: &'a str,
) -> impl Iterator<Item = poise::serenity_prelude::AutocompleteChoice> + 'a {
    let for_sale = ctx.data().roles.read().unwrap().clone();
    let roles = ctx
        .serenity_context()
        .http
        .get_guild_roles(ctx.guild_id().unwrap())
        .await
        .unwrap()
        .clone();

    roles
        .iter()
        .filter(move |cmd| for_sale.contains_key(&cmd.id))
        .map(|cmd| {
            poise::serenity_prelude::AutocompleteChoice::new(cmd.name.to_string(), cmd.to_string())
        })
        .collect::<Vec<poise::serenity_prelude::AutocompleteChoice>>()
        .into_iter()
}

async fn weekends_only(ctx: Context<'_>) -> Result<bool, Error> {
    if chrono::Utc::now().weekday() == chrono::Weekday::Sat
        || chrono::Utc::now().weekday() == chrono::Weekday::Sun
    {
        Ok(true)
    } else {
        ctx.send(
            CreateReply::default()
                .content("You can only buy on weekends!")
                .ephemeral(true)
                .reply(true),
        )
        .await?;
        Err("You can only buy on weekends!".into())
    }
}

async fn weekdays_only(ctx: Context<'_>) -> Result<bool, Error> {
    if chrono::Utc::now().weekday() == chrono::Weekday::Sat
        || chrono::Utc::now().weekday() == chrono::Weekday::Sun
    {
        ctx.send(
            CreateReply::default()
                .content("You can't do that on weekends!")
                .ephemeral(true)
                .reply(true),
        )
        .await?;
        Err("You can't do that on weekends!".into())
    } else {
        Ok(true)
    }
}

///
/// Buy bones with your JBucks
///
/// Enter `/buy bones <amount>`
/// ```
/// /buy bones 3
/// ```
#[poise::command(slash_command, check = "weekends_only")]
pub async fn bones(
    ctx: Context<'_>,
    #[description = "amount to purchase"]
    #[min = 1]
    #[max = 100]
    amount: Option<i32>,
) -> Result<(), Error> {
    let amount = amount.unwrap_or(1);
    let price = ctx.data().config.read().unwrap().bones_price;
    let balance = ctx.data().db.get_balance(ctx.author().id.get()).await?;
    if balance < price * amount {
        let reply = {
            CreateReply::default()
                .content(format!(
                    "You can't afford that many :bone:! You need {} {}!",
                    price * amount,
                    JBUCK_EMOJI
                ))
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("Not enough money".into());
    }

    ctx.data()
        .db
        .subtract_balances(vec![ctx.author().id.get()], price * amount)
        .await?;
    ctx.data()
        .db
        .add_bones(ctx.author().id.get(), amount)
        .await?;
    let reply = {
        CreateReply::default()
            .content(format!(
                "You have purchased {} :bone: for {} {}!",
                amount,
                price * amount,
                JBUCK_EMOJI
            ))
            .ephemeral(false)
    };
    ctx.send(reply).await?;
    Ok(())
}

///
/// Sell your bones
///
/// Enter `/sell bones <amount>`
/// ```
/// /sell bones 3
/// ```
#[poise::command(slash_command, rename = "bones", check = "weekdays_only")]
pub async fn sellbones(
    ctx: Context<'_>,
    #[description = "amount to sell"]
    #[min = 1]
    #[max = 100]
    amount: i32,
) -> Result<(), Error> {
    let balance = ctx.data().db.get_bones(ctx.author().id.get()).await?;
    if balance < amount {
        let reply = {
            CreateReply::default()
                .content("You don't have that many :bone:!".to_string())
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("Not enough bones".into());
    }
    let price = ctx.data().config.read().unwrap().bones_price;

    ctx.data()
        .db
        .award_balances(vec![ctx.author().id.get()], price * amount)
        .await?;
    ctx.data()
        .db
        .remove_bones(ctx.author().id.get(), amount)
        .await?;
    let reply = {
        CreateReply::default()
            .content(format!(
                "You have sold {} :bone: for {} {}!",
                amount,
                price * amount,
                JBUCK_EMOJI
            ))
            .ephemeral(false)
    };
    ctx.send(reply).await?;
    Ok(())
}

///
/// Buy an emoji for the server
///
/// Enter `/buy emoji`
/// ```
/// /buy emoji
/// ```
#[poise::command(slash_command)]
pub async fn emoji(
    ctx: Context<'_>,
    img: poise::serenity_prelude::Attachment,
) -> Result<(), Error> {
    ctx.defer().await?;
    let size = img.size;
    if size > 256 * 1024 {
        let reply = {
            CreateReply::default()
                .content("The image is too large! Please keep it under 256KB.")
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("Image too large".into());
    }

    let ct = img.content_type.clone();
    match ct {
        Some(ref ct) => {
            let allowed = ["image/png", "image/jpeg", "image/gif"];
            if !allowed.contains(&ct.as_str()) {
                let reply = {
                    CreateReply::default()
                        .content("That is not an image!")
                        .ephemeral(true)
                };
                ctx.send(reply).await?;
                return Err("Not an image".into());
            }
        }
        None => {
            let reply = {
                CreateReply::default()
                    .content("That is not an image!")
                    .ephemeral(true)
            };
            ctx.send(reply).await?;
            return Err("Not an image".into());
        }
    }

    let balance = ctx.data().db.get_balance(ctx.author().id.get()).await?;
    let price = { ctx.data().config.read().unwrap().community_emoji_price };
    if balance < price {
        let reply = {
            CreateReply::default()
                .content(format!(
                    "You can't afford that emoji! It costs {} {}!",
                    price, JBUCK_EMOJI
                ))
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("Not enough money".into());
    }

    ctx.data()
        .db
        .subtract_balances(vec![ctx.author().id.get()], price)
        .await?;

    let emoji = ctx.data().db.get_oldest_community_emoji().await.unwrap();

    let emojis: Vec<Emoji> = ctx
        .guild_id()
        .unwrap()
        .emojis(ctx)
        .await
        .unwrap()
        .iter()
        .filter(|a| a.name == emoji.name)
        .cloned()
        .collect();

    if !emojis.is_empty() {
        let emoji_id = emojis.first().unwrap();
        ctx.guild_id().unwrap().delete_emoji(ctx, emoji_id).await?;
    }

    let i = img.download().await.unwrap();
    let ct = ct.unwrap();

    let v = format!(
        "data:{};base64,{}",
        ct,
        general_purpose::STANDARD.encode(&i)
    );

    match ctx
        .guild_id()
        .unwrap()
        .create_emoji(ctx, &emoji.name, &v)
        .await
    {
        Ok(a) => {
            ctx.data().db.add_community_emoji(&emoji.name).await?;
            let reply = {
                CreateReply::default().content(format!(
                    "You have purchased the emoji <:{}:{}> for {} {}!",
                    a.name, a.id, price, JBUCK_EMOJI
                ))
            };
            ctx.send(reply).await?;
        }
        Err(e) => {
            tracing::debug!("{e}");
            ctx.data()
                .db
                .award_balances(vec![ctx.author().id.into()], price)
                .await?;
            let reply = {
                CreateReply::default()
                    .content("There was an error creating the emoji!")
                    .ephemeral(true)
            };
            // TODO: refund
            ctx.send(reply).await?;
        }
    };
    Ok(())
}

///
/// Buy a role with your JBucks
///
/// Enter `/buy role @role`
/// ```
/// /buy role @JohnnyBot
/// ```
#[poise::command(slash_command)]
pub async fn role(
    ctx: Context<'_>,
    #[description = "role to purchase"]
    #[autocomplete = "complete_roles"]
    role: poise::serenity_prelude::Role,
) -> Result<(), Error> {
    if !ctx.data().roles.read().unwrap().contains_key(&role.id) {
        let reply = {
            CreateReply::default()
                .content("That role is not for sale!")
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("Role not for sale".into());
    }

    // check if user has the role already
    if ctx
        .author()
        .has_role(ctx, ctx.guild_id().unwrap(), role.id)
        .await?
    {
        let reply = {
            CreateReply::default()
                .content("You already have that role!")
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("Role already owned".into());
    }

    let balance = ctx.data().db.get_balance(ctx.author().id.get()).await?;

    let price = { ctx.data().roles.read().unwrap()[&role.id] };

    if let Some(required_role) = price.1 {
        if !ctx
            .author()
            .has_role(ctx, ctx.guild_id().unwrap(), required_role)
            .await?
        {
            let reply = {
                CreateReply::default()
                    .content(format!(
                        "You need the role <@&{}> to purchase this role!",
                        required_role
                    ))
                    .ephemeral(true)
            };
            ctx.send(reply).await?;
            return Err("Missing required role".into());
        }
    }

    if balance < price.0 {
        let reply = {
            CreateReply::default()
                .content(format!(
                    "You can't afford that role! You need {} {}!",
                    price.0, JBUCK_EMOJI
                ))
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("Not enough money".into());
    }

    // give the user the role
    ctx.serenity_context()
        .http
        .add_member_role(
            ctx.guild_id().unwrap(),
            ctx.author().id,
            role.id,
            Some("Buying a role"),
        )
        .await?;

    ctx.data()
        .db
        .subtract_balances(vec![ctx.author().id.get()], price.0)
        .await?;

    if ctx.data().unique_roles.lock().unwrap().contains(&role.id) {
        if let Some(user) = ctx.data().db.get_unique_role_holder(role.id.into()).await? {
            let now = chrono::Utc::now();
            let bought = user.purchased;
            let time_since_purchase = now - bought;
            ctx.serenity_context()
                .http
                .remove_member_role(
                    ctx.guild_id().unwrap(),
                    poise::serenity_prelude::UserId::new(user.user_id),
                    role.id,
                    Some(format!("{} bought it", ctx.author().id).as_str()),
                )
                .await?;

            let v: f32 = time_since_purchase.num_minutes() as f32 / 60.0;
            ctx.data().db.update_crown_timer(user.user_id, v).await?;
        };
        ctx.data()
            .db
            .set_unique_role_holder(role.id.into(), ctx.author().id.into())
            .await?;

        ctx.data()
            .db
            .update_crown_timer(ctx.author().id.into(), 0.0)
            .await?;
    }

    incrementroleprice(ctx, role.id.to_string()).await?;

    let reply = {
        CreateReply::default().content(format!(
            "{} purchased {} for {} {}!",
            ctx.author(),
            role,
            price.0,
            JBUCK_EMOJI
        ))
    };
    ctx.send(reply).await?;

    Ok(())
}

///
/// Decay the price of a role
///
/// Enter `/decay @role [amount] [interval (hours)]`
/// ```
/// // decay the price of the role @JohnnyBot by 1 every 2 hours
/// /decay @JohnnyBot 1 2
/// ```
#[poise::command(
    slash_command,
    category = "Admin",
    default_member_permissions = "ADMINISTRATOR",
    hide_in_help
)]
pub async fn decay(
    ctx: Context<'_>,
    #[description = "role to decay"]
    #[autocomplete = "complete_roles"]
    role: poise::serenity_prelude::Role,
    #[min = 0]
    #[description = "The amount to decay the price by"]
    amount: i32,
    #[min = 1]
    #[description = "Interval in hours to perform the decay"]
    interval: i32,
    #[min = 1]
    #[description = "minimum allowed price for this role"]
    minimum: i32,
) -> Result<(), Error> {
    match ctx
        .data()
        .db
        .set_price_decay_config(role.id.into(), amount, interval, minimum)
        .await
    {
        Ok(_) => {
            let reply = {
                CreateReply::default()
                    .content(format!(
                        "You have set the decay for the role {} to -{} every {} hours!",
                        role, amount, interval
                    ))
                    .ephemeral(true)
            };
            ctx.send(reply).await?
        }
        Err(e) => {
            tracing::debug!("{e}");
            let reply = {
                CreateReply::default()
                    .content("There was an error setting the decay!\nTalk to Kidsan.")
                    .ephemeral(true)
            };
            ctx.send(reply).await?
        }
    };
    Ok(())
}

///
/// List the price decay config
///
#[poise::command(
    slash_command,
    category = "Admin",
    default_member_permissions = "ADMINISTRATOR",
    hide_in_help
)]
pub async fn list_decays(ctx: Context<'_>) -> Result<(), Error> {
    let config = ctx
        .data()
        .db
        .get_price_decay_config()
        .await?
        .iter()
        .map(|a| {
            format!(
                "> <@&{}> - {} every {} hours (minimum: {}, last: <t:{}:R>)",
                a.role_id,
                a.amount,
                a.interval,
                a.minimum,
                a.last_decay.naive_utc().and_utc().timestamp()
            )
        })
        .collect::<Vec<String>>()
        .join("\n");
    let reply = {
        CreateReply::default()
            .content(format!("### Price Decay Config ###\n\n{}", config))
            .ephemeral(true)
    };
    ctx.send(reply).await?;
    Ok(())
}

///
/// List the price config
///
#[poise::command(
    slash_command,
    category = "Admin",
    default_member_permissions = "ADMINISTRATOR",
    hide_in_help
)]
pub async fn list_prices(ctx: Context<'_>) -> Result<(), Error> {
    let config = ctx.data().db.get_purchasable_roles().await?;
    let embed = poise::serenity_prelude::CreateEmbed::new()
        .title("Price Config")
        .fields(vec![
            (
                "Role",
                config
                    .iter()
                    .map(|a| format!("<@&{}>", a.role_id.clone()))
                    .collect::<Vec<String>>()
                    .join("\n"),
                true,
            ),
            (
                "Increment",
                config
                    .iter()
                    .map(|a| a.increment.unwrap_or(0))
                    .map(|a| a.to_string())
                    .collect::<Vec<String>>()
                    .join("\n"),
                true,
            ),
            (
                "Prerequisite",
                config
                    .iter()
                    .map(|a| {
                        if let Some(b) = a.required_role_id {
                            format!("<@&{}>", b)
                        } else {
                            "None".to_string()
                        }
                    })
                    .collect::<Vec<String>>()
                    .join("\n"),
                true,
            ),
        ]);
    let reply = { CreateReply::default().ephemeral(true).embed(embed) };
    ctx.send(reply).await?;

    Ok(())
}

///
/// Get the status of the bones market
///
/// Enter `/bones`
/// ```
/// /bones
/// ```
#[poise::command(slash_command, rename = "bones")]
pub async fn bones_status(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await.unwrap();
    let price = ctx.data().config.read().unwrap().bones_price;
    let status = match is_weekend() {
        true => "> Status: **BUYING TIME :chart_with_upwards_trend: **",
        false => "> Status: **SELLING TIME :chart_with_downwards_trend: **",
    };
    let deadline = match is_weekend() {
        true => {
            // deadline is Monday 00:00 UTC
            let mut deadline = chrono::Utc::now();
            let now = chrono::Utc::now();
            if now.weekday() == chrono::Weekday::Sat {
                deadline = deadline.checked_add_days(Days::new(2)).unwrap()
            } else if now.weekday() == chrono::Weekday::Sun {
                deadline = deadline.checked_add_days(Days::new(1)).unwrap()
            }
            deadline = deadline
                .with_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
                .unwrap();

            format!("> Buying time ends: <t:{}:R>", deadline.timestamp())
        }
        false => {
            // deadline is Saturday 00:00 UTC
            let mut deadline = chrono::Utc::now();
            while deadline.weekday() != chrono::Weekday::Sat {
                tracing::debug!("{}", deadline.weekday());
                deadline = deadline.checked_add_days(Days::new(1)).unwrap();
            }
            deadline = deadline
                .with_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
                .unwrap();
            format!("> Bones expire: <t:{}:R>", deadline.timestamp())
        }
    };
    let footer = match is_weekend() {
        true => "> Buy more via** /buy bones** command!",
        false => "> Sell your stock via** /sell bones** command!",
    };
    let formatted_price = format!("> Price: **{}** {}", price, JBUCK_EMOJI);

    let lb = ctx.data().db.get_bones_leaderboard().await?;
    let named_players = {
        let mut map = std::collections::HashMap::new();
        for (player, _, _) in lb.clone() {
            let name = get_discord_name(ctx, player).await;
            map.insert(player, name);
        }
        map
    };
    let mut bones_leaderboard = lb
        .iter()
        .map(|(u, bones, _)| (named_players.get(u).unwrap(), bones))
        .map(|(u, b)| format!("> :bone: **{}** - {}\n", b, u))
        .collect::<Vec<String>>()
        .join("");

    if bones_leaderboard.is_empty() {
        bones_leaderboard = "> Nobody has any :bone: yet!".to_string();
    }
    let formatted_bones_leaderboard = format!("> Bone Holders:\n{}", bones_leaderboard.trim_end());
    let message = format!(
        "> **BONE MARKET**\n{status}\n{formatted_price}\n{deadline}\n{formatted_bones_leaderboard}\n{footer}"
    );
    let reply = { CreateReply::default().content(message).reply(true) };
    ctx.send(reply).await?;
    Ok(())
}

///
/// Sell something for JBucks
///
/// Enter `/sell `
/// ```
/// /sell bones 1
/// ```
#[poise::command(slash_command, subcommands("sellbones"), subcommand_required)]
pub async fn sell(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}
