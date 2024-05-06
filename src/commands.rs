use crate::{database::BalanceDatabase, Context, Error};
use poise::{
    serenity_prelude::{self as serenity, User},
    CreateReply,
};

#[poise::command(
    prefix_command,
    track_edits,
    slash_command,
    hide_in_help,
    category = "Admin",
    owners_only
)]
pub async fn register(ctx: Context<'_>) -> Result<(), Error> {
    poise::builtins::register_application_commands_buttons(ctx).await?;
    Ok(())
}

pub async fn complete_help<'a>(
    ctx: Context<'a>,
    partial: &'a str,
) -> impl Iterator<Item = serenity::AutocompleteChoice> + 'a {
    let white_listed = [
        "help",
        "balance",
        "leaderboard",
        "give",
        "coingamble",
        "daily",
        "bury",
        "buyrobbery",
        "rpsgamble",
        // "buy",
        // "shop"
    ];
    poise::builtins::autocomplete_command(ctx, partial)
        .await
        .filter(move |cmd| white_listed.contains(&cmd.as_str()))
        .map(|cmd| serenity::AutocompleteChoice::new(cmd.to_string(), cmd))
}

/// Show this help menu
#[poise::command(track_edits, slash_command)]
pub async fn help(
    ctx: Context<'_>,
    #[description = "Specific command to show help about"]
    #[autocomplete = "complete_help"]
    command: Option<String>,
) -> Result<(), Error> {
    if let Some(command) = &command {
        if ![
            "help",
            "balance",
            "leaderboard",
            "give",
            "coingamble",
            "daily",
            "bury",
            "buyrobbery",
            "rpsgamble",
            // "buy",
            // "shop"
        ]
        .contains(&command.as_str())
        {
            let reply = {
                CreateReply::default()
                    .content("Unknown command!")
                    .ephemeral(true)
            };
            ctx.send(reply).await?;
            return Ok(());
        }
    }
    poise::builtins::help(
        ctx,
        command.as_deref(),
        poise::builtins::HelpConfiguration {
            extra_text_at_bottom: "Awooo",
            ..Default::default()
        },
    )
    .await?;
    Ok(())
}

///
/// Check someone's balance
///
/// Enter `/checkbucks @Name` to check
/// ```
/// /checkbucks @John
/// ```
///
#[poise::command(
    slash_command,
    category = "Admin",
    default_member_permissions = "ADMINISTRATOR",
    hide_in_help
)]
pub async fn checkbucks(
    ctx: Context<'_>,
    #[description = "Who to check"] user: serenity::User,
) -> Result<(), Error> {
    let user_id = user.id.to_string();
    let response = match user.bot {
        true => 0,
        false => ctx.data().db.get_balance(user_id).await?,
    };
    let reply = {
        CreateReply::default()
            .content(format!("{} has {} J-Buck(s)!", user, response,))
            .ephemeral(true)
    };
    ctx.send(reply).await?;
    Ok(())
}

///
/// Have Johnny say something
///
/// Enter `/say <message>` to make Johnny say something
/// ```
/// /say Awoo
/// ```
///
#[poise::command(
    slash_command,
    category = "Admin",
    default_member_permissions = "ADMINISTRATOR",
    hide_in_help
)]
pub async fn say(
    ctx: Context<'_>,
    #[description = "What to say?"] message: String,
) -> Result<(), Error> {
    let reply = { CreateReply::default().content("Success!").ephemeral(true) };
    ctx.send(reply).await?;
    ctx.channel_id().say(ctx, message).await?;
    Ok(())
}

///
/// Check your balance
///
/// Enter `/balance` to check
/// ```
/// /balance
/// ```
#[poise::command(slash_command)]
pub async fn balance(ctx: Context<'_>) -> Result<(), Error> {
    let user_id = ctx.author().id.to_string();
    let response = ctx.data().db.get_balance(user_id).await?;
    let reply = {
        CreateReply::default()
            .content(format!("{} has {} J-Buck(s)!", ctx.author(), response,))
            .ephemeral(true)
    };
    ctx.send(reply).await?;
    Ok(())
}

fn user_can_play(user_balance: i32, amount: i32) -> bool {
    user_balance >= amount
}

///
/// Give some bucks to another player
///
/// Enter `/give <recipient> <amount>`
/// ```
/// /give @John 50
/// ```
#[poise::command(slash_command)]
pub async fn give(
    ctx: Context<'_>,
    #[description = "Who to send to"] recipient: User,
    #[min = 1]
    #[description = "How much to send"]
    amount: i32,
) -> Result<(), Error> {
    if recipient.id.to_string() == ctx.author().id.to_string() {
        let reply = {
            CreateReply::default()
                .content("Don't send money to yourself..")
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("You can't afford to do that".into());
    }
    if recipient.bot {
        let reply = {
            CreateReply::default()
                .content("You can't send money to bots..")
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("You can't afford to do that".into());
    }
    let sender = ctx.author().id.to_string();
    let db = &ctx.data().db;
    let sender_balance = ctx.data().db.get_balance(sender.clone()).await?;
    let recipient_id = recipient.id.to_string();
    let recipient_balance = ctx.data().db.get_balance(recipient_id.clone()).await?;
    if !user_can_play(sender_balance, amount) {
        let reply = {
            CreateReply::default()
                .content(format!(
                    "You can't afford to do that!\nYour balance is only {} J-Buck(s)",
                    sender_balance
                ))
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("You can't afford to do that".into());
    }

    let tax = amount as f32 * 0.02;
    // round tax up to the nearest integer
    let tax = tax.ceil() as i32;

    db.set_balance(sender.clone(), sender_balance - amount)
        .await?;
    db.set_balance(recipient_id.clone(), recipient_balance + (amount - tax))
        .await?;
    let reply = {
        CreateReply::default().content(format!(
            "{} sent {} <:jbuck:1228663982462865450> to {}!\n -{} <:jbuck:1228663982462865450> Johnny's work fee.",
            ctx.author(),
            amount,
            recipient,
            tax,
        ))
    };
    ctx.send(reply).await?;
    Ok(())
}

///
/// Remove bucks from a player
///
/// ```
/// /remove_bucks @John 50
/// ```
#[poise::command(
    slash_command,
    category = "Admin",
    hide_in_help,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn remove_bucks(
    ctx: Context<'_>,
    #[description = "Who to remove from"] user: User,
    #[min = 1]
    #[description = "How much to remove"]
    amount: i32,
) -> Result<(), Error> {
    if user.bot {
        let reply = {
            CreateReply::default()
                .content("You can't remove money from bots..")
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("You can't afford to do that".into());
    }
    let user_id = user.id.to_string();
    let user_balance = ctx.data().db.get_balance(user_id.clone()).await?;
    if !user_can_play(user_balance, amount) {
        let reply = {
            CreateReply::default()
                .content(format!(
                    "They can't afford to do that!\n{}'s balance is only {} J-Buck(s)",
                    user, user_balance
                ))
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("can't afford to do that".into());
    }
    ctx.data()
        .db
        .set_balance(user_id.clone(), user_balance - amount)
        .await?;

    let reply =
        { CreateReply::default().content(format!("Removed {} J-Bucks from {}", amount, user,)) };
    ctx.send(reply).await?;
    Ok(())
}

///
/// Fine a player
///
/// Enter `/fine <player> <amount>`
/// ```
/// /fine @John 50
/// ```
///
#[poise::command(
    slash_command,
    category = "Admin",
    hide_in_help,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn fine(
    ctx: Context<'_>,
    #[description = "Who to fine"] user: User,
    #[min = 1]
    #[description = "How much to fine them"]
    amount: i32,
    #[description = "Show that you invoked the command?"] show_caller: Option<bool>,
    #[description = "Reason for fine"] reason: Option<String>,
) -> Result<(), Error> {
    let user_id = user.id.to_string();
    let user_balance = ctx.data().db.get_balance(user_id.clone()).await?;
    if !user_can_play(user_balance, amount) {
        let reply = {
            CreateReply::default()
                .content(format!(
                    "They can't afford to do that!\n{}'s balance is only {} J-Bucks",
                    user, user_balance
                ))
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("Can't afford to do that".into());
    }
    ctx.data()
        .db
        .set_balance(user_id.clone(), user_balance - amount)
        .await?;

    let msg = match reason {
        Some(r) => format!(
            "{} was fined {} <:jbuck:1228663982462865450>!\nReason: \"*{}*\"",
            user, amount, r
        ),
        None => format!(
            "{} was fined {} <:jbuck:1228663982462865450>!",
            user, amount
        ),
    };

    // if show_caller is true, send as a reply
    match show_caller {
        Some(true) => {
            let reply = { CreateReply::default().content(msg) };
            ctx.send(reply).await?;
        }
        _ => {
            // acknowledge the invocation
            ctx.send(CreateReply::default().content("success").ephemeral(true))
                .await?;
            ctx.channel_id().say(ctx, msg).await?;
        }
    }
    Ok(())
}

///
/// award bucks to a player
///
/// Enter `/award <player> <amount>`
/// ```
/// /award @John 50
/// ```
#[poise::command(
    slash_command,
    category = "Admin",
    hide_in_help,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn award(
    ctx: Context<'_>,
    #[description = "Who to award"] user: User,
    #[min = 1]
    #[description = "How much to award"]
    amount: i32,
    #[description = "Show that you invoked the command?"] show_caller: Option<bool>,
    #[description = "Reason for award"] reason: Option<String>,
) -> Result<(), Error> {
    if user.bot {
        let reply = {
            CreateReply::default()
                .content("You can't award bots..")
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("You can't afford to do that".into());
    }
    let user_id = user.id.to_string();
    let user_balance = ctx.data().db.get_balance(user_id.clone()).await?;
    ctx.data()
        .db
        .set_balance(user_id.clone(), user_balance + amount)
        .await?;

    // if show_caller is true, send as a reply
    let msg = match reason {
        Some(m) => format!(
            "{} was awarded {} <:jbuck:1228663982462865450>!\nReason: \"*{}*\"",
            user, amount, m
        ),
        None => format!(
            "{} was awarded {} <:jbuck:1228663982462865450>!",
            user, amount
        ),
    };
    match show_caller {
        Some(true) => {
            let reply = { CreateReply::default().content(msg) };
            ctx.send(reply).await?;
        }
        _ => {
            // acknowledge the invocation
            ctx.send(CreateReply::default().content("success").ephemeral(true))
                .await?;
            ctx.channel_id().say(ctx, msg).await?;
        }
    }
    Ok(())
}

///
/// add bucks to a player
///
/// Enter `/add_bucks <player> <amount>`
/// ```
/// /add_bucks @John 50
/// ```
#[poise::command(
    slash_command,
    category = "Admin",
    hide_in_help,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn add_bucks(
    ctx: Context<'_>,
    #[description = "Who to give bucks to"] user: User,
    #[min = 1]
    #[description = "How much to add"]
    amount: i32,
) -> Result<(), Error> {
    if user.bot {
        let reply = {
            CreateReply::default()
                .content("You can't add money to bots..")
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("You can't afford to do that".into());
    }
    let user_id = user.id.to_string();
    let user_balance = ctx.data().db.get_balance(user_id.clone()).await?;
    ctx.data()
        .db
        .set_balance(user_id.clone(), user_balance + amount)
        .await?;
    let reply =
        { CreateReply::default().content(format!("{} was given {} J-Buck(s)", user, amount,)) };
    ctx.send(reply).await?;
    Ok(())
}

///
/// Transfer some bucks between players
///
/// Enter `/transfer <source> <recipient> <amount>`
/// ```
/// /transfer @John @Adam 50
/// ```
#[poise::command(
    slash_command,
    category = "Admin",
    hide_in_help,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn transfer(
    ctx: Context<'_>,
    #[description = "Who to remove from"] source: User,
    #[description = "Who to give to"] recipient: User,
    #[min = 1]
    #[description = "How much to transfer"]
    amount: i32,
) -> Result<(), Error> {
    if source.id == recipient.id {
        let reply = {
            CreateReply::default()
                .content("No action required")
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("No action required".into());
    }
    if source.bot || recipient.bot {
        let reply = {
            CreateReply::default()
                .content("You can't transfer money to or from bots..")
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("You can't afford to do that".into());
    }
    let user_id = source.id.to_string();
    let user_balance = ctx.data().db.get_balance(user_id.clone()).await?;
    if !user_can_play(user_balance, amount) {
        let reply = {
            CreateReply::default()
                .content(format!(
                    "They can't afford to do that!\n{}'s balance is only {} J-Buck(s)",
                    source, user_balance
                ))
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("can't afford to do that".into());
    }
    let recipient_id = recipient.id.to_string();
    let recipient_balance = ctx.data().db.get_balance(recipient_id.clone()).await?;
    ctx.data()
        .db
        .set_balance(user_id.clone(), user_balance - amount)
        .await?;
    ctx.data()
        .db
        .set_balance(recipient_id.clone(), recipient_balance + amount)
        .await?;

    let reply = {
        CreateReply::default().content(format!(
            "Removed {} J-Buck(s) from {} and gave it to {}",
            amount, source, recipient
        ))
    };
    ctx.send(reply).await?;
    Ok(())
}
