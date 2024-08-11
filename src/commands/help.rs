use crate::{Context, Error};
use poise::serenity_prelude as serenity;
use poise::CreateReply;

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
        "buy",
        "sell",
        "bones",
        "shop",
        "lottery",
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
            "sellbones",
            "buy",
            "shop",
            "lottery",
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
