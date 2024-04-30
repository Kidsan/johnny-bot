use std::time::{SystemTime, UNIX_EPOCH};

use crate::{database::BalanceDatabase, Context, Error};
use poise::CreateReply;

use ::poise::serenity_prelude::{self as serenity};
use ::serenity::all::CreateMessage;

#[derive(Debug, poise::ChoiceParameter, Clone)]
pub enum RPSChoice {
    Rock,
    Paper,
    Scissors,
}

impl std::fmt::Display for RPSChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RPSChoice::Rock => write!(f, ":rock:"),
            RPSChoice::Paper => write!(f, ":roll_of_paper:"),
            RPSChoice::Scissors => write!(f, ":scissors:"),
        }
    }
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
    if user.bot && user.id.to_string() != ctx.data().bot_id {
        let reply = {
            CreateReply::default()
                .content("You can't play against a bot, they have no hands")
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("You can't do that".into());
    }
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let time_to_play = ctx.data().game_length;
    if user.id == ctx.author().id {
        let reply = { CreateReply::default().content("You can't challenge yourself!") };
        ctx.send(reply).await?;
        return Err("Can't challenge yourself".into());
    }
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
                    "You can't afford to bet {}. You only have {} <:jbuck:1228663982462865450>!",
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

    let components = match user.id.to_string() == ctx.data().bot_id {
        true => vec![],
        false => vec![serenity::CreateActionRow::Buttons(vec![
            new_rock_button(),
            new_paper_button(),
            new_scissors_button(),
        ])],
    };

    let reply = {
        CreateMessage::default()
            .content(format!(
                "{} has challenged {} to a game of :rock: :roll_of_paper: :scissors: for {} <:jbuck:1228663982462865450>!",
                ctx.author(),
                user,
                amount
            ))
            .components(components)
    };

    let mut message = ctx.channel_id().send_message(ctx, reply).await?;

    if user.id.to_string() == ctx.data().bot_id {
        let reply = {
            CreateMessage::default()
                .content(format!(
                    "I win! {}",
                    crate::newcommands::blackjack::get_troll_emoji()
                ))
                .reference_message(&message)
        };
        ctx.channel_id().send_message(ctx, reply).await?;
        return Ok(());
    }

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
            mci.create_response(
                ctx,
                serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::new()
                        .content(format!("You are not {}", user))
                        .allowed_mentions(serenity::CreateAllowedMentions::new().empty_users())
                        .ephemeral(true),
                ),
            )
            .await?;
            continue;
        }
        let balance = { ctx.data().db.get_balance(user.id.to_string()).await? };
        if amount > balance {
            let content = message.content.clone();
            message
                .edit(
                    ctx,
                    serenity::EditMessage::new()
                        .content(content)
                        .components(vec![]),
                )
                .await?;
            mci.create_response(
                ctx,
                serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::new()
                        .allowed_mentions(serenity::CreateAllowedMentions::new().empty_users())
                    .content(format!(
                        "You can't afford to play {}. You only have {} <:jbuck:1228663982462865450>!",
                        amount, balance
                    ))
                        .ephemeral(true),
                ),
            )
            .await?;
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
            ctx.data()
                .db
                .award_balances(vec![ctx.author().id.to_string()], amount)
                .await?;
            let content = message.content.clone();

            message
                .edit(
                    ctx,
                    serenity::EditMessage::new()
                        .content(content)
                        .components(vec![]),
                )
                .await?;
            let msg = {
                CreateMessage::default()
                    .content(format!(
                        "{} did not respond in time! You get your money back!",
                        user
                    ))
                    .reference_message(&message)
            };
            ctx.channel_id().send_message(ctx, msg).await?;
            return Err("{} did not respond in time".into());
        }
    };
    let result = (user_choice - challengee_value + 3) % 3;
    let msg = match result {
        0 => {
            ctx.data()
                .db
                .award_balances(vec![ctx.author().id.to_string()], amount)
                .await?;
            format!(
                "{} and {} both chose {}\nit is a tie! **Refunds all around**",
                ctx.author(),
                user,
                choice
            )
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

            format!(
                "{} chose {}, {} chose {}\n{} {}! **They get {} **<:jbuck:1228663982462865450>",
                ctx.author(),
                choice,
                user,
                challengee_choice.unwrap(),
                ctx.author(),
                "won",
                amount * 2
            )
        }
        2 => {
            ctx.data()
                .db
                .award_balances(vec![user.id.to_string()], amount)
                .await?;
            format!(
                "{} chose {}, {} chose {}\n{} {}! **They get {}** <:jbuck:1228663982462865450>",
                ctx.author(),
                choice,
                user,
                challengee_choice.unwrap(),
                user,
                "won",
                amount * 2
            )
        }
        _ => unreachable!(),
    };

    let content = message.content.clone();

    message
        .edit(
            ctx,
            serenity::EditMessage::new()
                .content(content)
                .components(vec![]),
        )
        .await?;

    let reply = {
        CreateMessage::default()
            .content(msg)
            .reference_message(&message)
    };
    ctx.channel_id().send_message(ctx, reply).await?;
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
