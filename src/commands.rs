use std::{
    time::{Duration, SystemTime, UNIX_EPOCH},
    vec,
};

use crate::{Context, Error};
use poise::{serenity_prelude as serenity, CreateReply};

#[poise::command(prefix_command, track_edits, slash_command)]
pub async fn register(ctx: Context<'_>) -> Result<(), Error> {
    poise::builtins::register_application_commands_buttons(ctx).await?;
    Ok(())
}

/// Show this help menu
#[poise::command(prefix_command, track_edits, slash_command)]
pub async fn help(
    ctx: Context<'_>,
    #[description = "Specific command to show help about"]
    #[autocomplete = "poise::builtins::autocomplete_command"]
    command: Option<String>,
) -> Result<(), Error> {
    poise::builtins::help(
        ctx,
        command.as_deref(),
        poise::builtins::HelpConfiguration {
            extra_text_at_bottom: "This is an example bot made to showcase features of my custom Discord bot framework",
            ..Default::default()
        },
    )
    .await?;
    Ok(())
}

/// Check your balance
///
/// Enter `~checkbucks` to check
/// ````
/// /checkbucks
/// ```
#[poise::command(prefix_command, slash_command)]
pub async fn checkbucks(ctx: Context<'_>) -> Result<(), Error> {
    let response = format!("{} has {} J-Bucks!", ctx.author(), 50);
    ctx.say(response).await?;
    Ok(())
}

/// Start a gamble
///
/// Enter `~startGamble` to play
/// ````
/// /startGamble
/// ```
#[poise::command(prefix_command, track_edits, slash_command)]
pub async fn start_gamble(
    ctx: Context<'_>,
    #[description = "amount to play"] amount: i32,
) -> Result<(), Error> {
    let mut players = vec![ctx.author().to_string()];
    let mut pot = amount;
    let components = vec![serenity::CreateActionRow::Buttons(vec![
        serenity::CreateButton::new("Bet")
            .label(format!("Bet {} J-Bucks", amount))
            .style(poise::serenity_prelude::ButtonStyle::Success),
        serenity::CreateButton::new("Players")
            .label(format!("Players: {}", players.len()))
            .disabled(true)
            .style(poise::serenity_prelude::ButtonStyle::Secondary),
        serenity::CreateButton::new("Pot")
            .label(format!("Total Pot: {}", pot))
            .disabled(true)
            .style(poise::serenity_prelude::ButtonStyle::Danger),
    ])];
    let reply = {
        CreateReply::default()
            .content(format!(
                "{} has started a game, place your bets!\n Betting deadline <t:{}:R>",
                ctx.author(),
                SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + 300
            ))
            .components(components.clone())
    };

    ctx.send(reply).await?;
    while let Some(mci) = serenity::ComponentInteractionCollector::new(ctx)
        .author_id(ctx.author().id)
        .channel_id(ctx.channel_id())
        .timeout(std::time::Duration::from_secs(10))
        // .filter(move |mci| mci.data.custom_id == game_id.to_string())
        .await
    {
        players.push(mci.user.to_string());
        pot += amount;

        let mut msg = mci.message.clone();

        msg.edit(
            ctx,
            serenity::EditMessage::new().components(vec![serenity::CreateActionRow::Buttons(
                vec![
                    serenity::CreateButton::new("Bet")
                        .label(format!("Bet {} J-Bucks", amount))
                        .style(poise::serenity_prelude::ButtonStyle::Success),
                    serenity::CreateButton::new("Players")
                        .label(format!("Players: {}", players.len()))
                        .disabled(true)
                        .style(poise::serenity_prelude::ButtonStyle::Secondary),
                    serenity::CreateButton::new("Pot")
                        .label(format!("Total Pot: {}", pot))
                        .disabled(true)
                        .style(poise::serenity_prelude::ButtonStyle::Danger),
                ],
            )]),
        )
        .await?;

        ctx.reply(format!("{} has entered a bet!", mci.user))
            .await?;

        mci.create_response(ctx, serenity::CreateInteractionResponse::Acknowledge)
            .await?;
    }
    Ok(())
}

/// View Leaderboard
///
/// Enter `~leaderboard` to view
/// ````
/// /leaderboard
/// ```
#[poise::command(prefix_command, slash_command)]
pub async fn leaderboard(ctx: Context<'_>) -> Result<(), Error> {
    let mut response = String::new();
    for i in 0..9 {
        response += format!("{}. {} with {} J-Bucks!\n", i + 1, ctx.author(), 50).as_str();
    }
    ctx.say(response).await?;
    Ok(())
}

/// Vote for something
///
/// Enter `~vote pumpkin` to vote for pumpkins
#[poise::command(prefix_command, slash_command)]
pub async fn vote(
    ctx: Context<'_>,
    #[description = "What to vote for"] choice: String,
) -> Result<(), Error> {
    // Lock the Mutex in a block {} so the Mutex isn't locked across an await point
    let num_votes = {
        let mut hash_map = ctx.data().votes.lock().unwrap();
        let num_votes = hash_map.entry(choice.clone()).or_default();
        *num_votes += 1;
        *num_votes
    };

    let response = format!("Successfully voted for {choice}. {choice} now has {num_votes} votes!");
    ctx.say(response).await?;
    Ok(())
}

/// Retrieve number of votes
///
/// Retrieve the number of votes either in general, or for a specific choice:
/// ```
/// ~getvotes
/// ~getvotes pumpkin
/// ```
#[poise::command(prefix_command, track_edits, aliases("votes"), slash_command)]
pub async fn getvotes(
    ctx: Context<'_>,
    #[description = "Choice to retrieve votes for"] choice: Option<String>,
) -> Result<(), Error> {
    if let Some(choice) = choice {
        let num_votes = *ctx.data().votes.lock().unwrap().get(&choice).unwrap_or(&0);
        let response = match num_votes {
            0 => format!("Nobody has voted for {} yet", choice),
            _ => format!("{} people have voted for {}", num_votes, choice),
        };
        ctx.say(response).await?;
    } else {
        let mut response = String::new();
        for (choice, num_votes) in ctx.data().votes.lock().unwrap().iter() {
            response += &format!("{}: {} votes", choice, num_votes);
        }

        if response.is_empty() {
            response += "Nobody has voted for anything yet :(";
        }

        ctx.say(response).await?;
    };

    Ok(())
}
