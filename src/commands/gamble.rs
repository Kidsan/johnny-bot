pub(crate) use crate::commands::coingamble::new_pot_counter_button;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    commands::coingamble::new_player_count_button, database::BalanceDatabase, Context, Error,
};
use poise::{serenity_prelude as serenity, CreateReply};
///
/// Start a gamble
///
/// Enter `/gamble <amount>` to play
/// ```
/// /gamble 20
/// ```
#[poise::command(
    track_edits,
    slash_command,
    category = "Admin",
    default_member_permissions = "ADMINISTRATOR",
    hide_in_help
)]
pub async fn gamble(
    ctx: Context<'_>,
    #[description = "amount to play"]
    #[min = 1]
    amount: i32,
) -> Result<(), Error> {
    let game_length = { ctx.data().config.read().unwrap().game_length_seconds };
    let game_starter = ctx.author().id.to_string();
    let db = &ctx.data().db;
    let user_balance = db.get_balance(ctx.author().id.get()).await?;
    if amount > user_balance {
        let reply = {
            CreateReply::default()
                .content(format!(
                    "You can't afford to do that!\nYour balance is only {} J-Buck(s)",
                    user_balance
                ))
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("You can't afford to do that".into());
    }
    db.subtract_balances(vec![game_starter.parse().unwrap()], amount)
        .await?;

    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let time_to_play = game_length;
    let players = [game_starter.clone()];
    let pot = amount;
    let components = vec![serenity::CreateActionRow::Buttons(vec![
        new_bet_button(amount),
        new_player_count_button(players.len() as i32),
        new_pot_counter_button(pot),
    ])];
    let reply = {
        CreateReply::default()
            .content(format!(
                "{} has started a game, place your bets!\n Betting deadline <t:{}:R>",
                ctx.author(),
                now + time_to_play as u64
            ))
            .components(components.clone())
    };

    let a = ctx.send(reply).await?;
    let id = a.message().await?.id;
    {
        let mut games = ctx.data().games.lock().unwrap();
        games.insert(
            id.to_string(),
            crate::game::Game::new(amount, ctx.author().id.get()),
        );
    }

    while let Some(mci) = serenity::ComponentInteractionCollector::new(ctx)
        .channel_id(ctx.channel_id())
        .message_id(id)
        .timeout(std::time::Duration::from_secs(
            (now + time_to_play as u64 - 1)
                - SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        ))
        .filter(move |mci| mci.data.custom_id == "Bet")
        .await
    {
        let player = mci.user.id.to_string();
        {
            if ctx
                .data()
                .games
                .lock()
                .unwrap()
                .get(&id.to_string())
                .unwrap()
                .players
                .contains(&(mci.user.id.get()))
            {
                mci.create_response(ctx, serenity::CreateInteractionResponse::Acknowledge)
                    .await?;
                continue;
            }
        }

        let player_balance = db.get_balance(mci.user.id.get()).await?;

        if amount > player_balance {
            mci.create_response(
                ctx,
                serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::new()
                        .content(format!(
                            "You can't afford to do that!\nYour balance is only {} J-Buck(s)",
                            user_balance
                        ))
                        .ephemeral(true),
                ),
            )
            .await?;
            continue;
        }
        db.subtract_balances(vec![player.parse().unwrap()], amount)
            .await?;

        let button2;
        let button3;
        {
            let mut games = ctx.data().games.lock().unwrap();
            let game = games.get_mut(&id.to_string()).unwrap();
            game.player_joined(mci.user.id.get());
            button2 = new_player_count_button(game.players.len() as i32);
            button3 = new_pot_counter_button(game.pot);
        }

        let mut msg = mci.message.clone();

        msg.edit(
            ctx,
            serenity::EditMessage::new().components(vec![serenity::CreateActionRow::Buttons(
                vec![new_bet_button(amount), button2, button3],
            )]),
        )
        .await?;

        mci.create_response(ctx, serenity::CreateInteractionResponse::Acknowledge)
            .await?;
    }

    let game = {
        let mut games = ctx.data().games.lock().unwrap();
        games.remove(&id.to_string()).unwrap()
    };
    let winner = game.get_winner(&mut ctx.data().rng.lock().unwrap());
    let button2 = new_player_count_button(game.players.len() as i32);
    let button3 = new_pot_counter_button(game.pot);
    let prize = game.pot;

    // let winner_id = winner.parse().unwrap();

    db.award_balances(vec![winner], prize).await?;
    // let winner_id = winner.parse().unwrap();
    a.edit(
        ctx,
        CreateReply::default()
            .content(format!(
                "Game is over, winner is: {}, they won: {} J-Buck(s)!",
                serenity::UserId::new(winner as u64).to_user(ctx).await?,
                prize
            ))
            .components(vec![serenity::CreateActionRow::Buttons(vec![
                new_bet_button(amount).disabled(true),
                button2,
                button3,
            ])]),
    )
    .await?;
    // TODO: see if we can find a nice way to tell the user their balance after they win
    Ok(())
}

fn new_bet_button(amount: i32) -> serenity::CreateButton {
    serenity::CreateButton::new("Bet")
        .label(format!("Bet {} J-Buck(s)", amount))
        .style(poise::serenity_prelude::ButtonStyle::Primary)
}
