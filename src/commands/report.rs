use crate::{Context, Error};
use poise::CreateReply;

///
/// report an issue
///
/// Enter `/report <issue>` to report an issue to the bot developers.
/// ```
/// /report my gamble at 15:00 didn't work
/// ```
#[poise::command(slash_command)]
pub async fn report(
    ctx: Context<'_>,
    #[description = "What is your issue?"]
    #[min_length = 5]
    #[max_length = 50]
    issue: String,
    #[description = "Optional link to relevant discord message"] link: Option<String>,
) -> Result<(), Error> {
    ctx.data()
        .db
        .save_report(ctx.author().id.get(), issue, link)
        .await?;
    let reply = {
        CreateReply::default().content("Thank you for reporting this issue.\nYour issue is important to us.\nA support agent will be assigned to look at this issue and resolve it as soon as possible.").ephemeral(false)
    };
    ctx.send(reply).await?;
    Ok(())
}

///
/// Get reports
///
/// Enter `/reports` to list all the reports.
/// ```
/// /reports
/// ```
#[poise::command(
    slash_command,
    category = "Admin",
    hide_in_help,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn reports(ctx: Context<'_>) -> Result<(), Error> {
    let reports = ctx.data().db.get_reports().await?;
    let report_text = reports
        .iter()
        .map(|report| {
            format!(
                "ID: {} Reported by: <@{}>\nIssue: {}\nLink: {}\n",
                report.0,
                report.1,
                report.2,
                report.3.clone().unwrap_or("None".to_string())
            )
        })
        .collect::<Vec<String>>()
        .join("\n");

    let reply = {
        CreateReply::default()
            .content(format!("Reports:\n{}", report_text))
            .ephemeral(true)
    };
    ctx.send(reply).await?;
    Ok(())
}
