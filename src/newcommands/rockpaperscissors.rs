use std::time::{SystemTime, UNIX_EPOCH};

use crate::{database::BalanceDatabase, Context, Error};
use poise::CreateReply;

use ::poise::serenity_prelude::{
    self as serenity, CreateAllowedMentions, CreateInteractionResponseMessage, User,
};
use ::serenity::all::CreateMessage;

#[derive(Debug, poise::ChoiceParameter, Clone)]
pub enum RPSChoice {
    Rock,
    Paper,
    Scissors,
}

///
/// Play a game of Rock, Paper, Scissors with someone
///
/// Enter `/rockpaperscissors <amount> @John` to challenge someone to a game of rock, paper, scissors.
/// ```
/// /rockpaperscissors 10 @John
/// ```
#[poise::command(slash_command)]
pub async fn rockpaperscissors(
    ctx: Context<'_>,
    #[description = "The amount of J-Bucks to bet"]
    #[min = 0]
    amount: i32,
    #[description = "Who to challenge"] user: poise::serenity_prelude::User,
    #[description = "Your choice"] choice: RPSChoice,
) -> Result<(), Error> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let time_to_play = ctx.data().game_length;
    let balance = {
        ctx.data()
            .db
            .get_balance(ctx.author().id.to_string())
            .await?
    };
    if amount > balance {
        let reply = {
            CreateReply::default()
                .content(format!(
                    "You can't afford to bet {}. You only have {} :dollar:!",
                    amount, balance
                ))
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("Not enough money".into());
    }

    {
        ctx.data()
            .db
            .subtract_balances(vec![ctx.author().id.to_string()], amount)
            .await?;
    }

    ctx.send(CreateReply::default().content("success").ephemeral(true))
        .await?;

    let reply = {
        CreateMessage::default()
            .content(format!(
                "{} has challenged {} to a game of Rock, Paper, Scissors for {} :dollar:!",
                ctx.author(),
                user,
                amount
            ))
            .components(vec![serenity::CreateActionRow::Buttons(vec![
                new_rock_button(),
                new_paper_button(),
                new_scissors_button(),
            ])])
    };

    let message = ctx.channel_id().send_message(ctx, reply).await?;

    let user_choice = match choice {
        RPSChoice::Rock => 0,
        RPSChoice::Paper => 1,
        RPSChoice::Scissors => 2,
    };

    let mut challengee_choice: Option<RPSChoice> = None;

    while let Some(mci) = serenity::ComponentInteractionCollector::new(ctx)
        .channel_id(ctx.channel_id())
        .custom_ids(vec![
            "rock".to_string(),
            "paper".to_string(),
            "scissors".to_string(),
        ])
        .message_id(message.id)
        .timeout(std::time::Duration::from_secs(
            (now + time_to_play - 1) - SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        ))
        .await
    {
        if mci.user.id != user.id {
            mci.create_response(ctx, serenity::CreateInteractionResponse::Acknowledge)
                .await?;
            continue;
        }
        let balance = { ctx.data().db.get_balance(user.id.to_string()).await? };
        if amount > balance {
            let reply = {
                CreateReply::default()
                    .content(format!(
                        "You can't afford to bet {}. You only have {} :dollar:!",
                        amount, balance
                    ))
                    .ephemeral(true)
            };
            ctx.send(reply).await?;
            return Err("Not enough money".into());
        }
        challengee_choice = match mci.data.custom_id.to_string().as_str() {
            "rock" => Some(RPSChoice::Rock),
            "paper" => Some(RPSChoice::Paper),
            "scissors" => Some(RPSChoice::Scissors),
            _ => unreachable!(),
        };
        // acknowledge the interaction
        mci.create_response(ctx, serenity::CreateInteractionResponse::Acknowledge)
            .await?;
        break;
    }
    let challengee_value = match challengee_choice {
        Some(RPSChoice::Rock) => 0,
        Some(RPSChoice::Paper) => 1,
        Some(RPSChoice::Scissors) => 2,
        None => {
            let reply = { CreateReply::default().content("Challengee did not respond in time!") };
            ctx.send(reply).await?;
            return Err("Challengee did not respond in time".into());
        }
    };
    let result = (user_choice - challengee_value + 3) % 3;
    match result {
        0 => {
            ctx.data()
                .db
                .award_balances(vec![ctx.author().id.to_string()], amount)
                .await?;
        }
        1 => {
            ctx.data()
                .db
                .award_balances(vec![ctx.author().id.to_string()], amount * 2)
                .await?;
            ctx.data()
                .db
                .subtract_balances(vec![user.id.to_string()], amount)
                .await?;
        }
        2 => {
            ctx.data()
                .db
                .award_balances(vec![user.id.to_string()], amount)
                .await?;
        }
        _ => unreachable!(),
    };
    let reply = {
        CreateReply::default().content(format!(
            "challenger chose {:?}, challengee chose {:?}. challengee {}!",
            choice,
            challengee_choice.unwrap(),
            match result {
                0 => "tied",
                1 => "lost",
                2 => "won",
                _ => unreachable!(),
            }
        ))
    };
    ctx.send(reply).await?;
    Ok(())
}

fn new_rock_button() -> serenity::CreateButton {
    serenity::CreateButton::new("rock")
        .label("Rock")
        .style(poise::serenity_prelude::ButtonStyle::Primary)
}
fn new_paper_button() -> serenity::CreateButton {
    serenity::CreateButton::new("paper")
        .label("Paper")
        .style(poise::serenity_prelude::ButtonStyle::Primary)
}
fn new_scissors_button() -> serenity::CreateButton {
    serenity::CreateButton::new("scissors")
        .label("Scissors")
        .style(poise::serenity_prelude::ButtonStyle::Primary)
}
