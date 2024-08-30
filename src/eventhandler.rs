use crate::database::BalanceDatabase;
use crate::discord::{EGG_ROLE, NICKNAME_LICENCE};
use crate::{Data, Error};
use ::serenity::all::{EditMember, RoleId};
use poise::serenity_prelude as serenity;
use serenity::Result;

pub async fn event_handler(
    ctx: &serenity::Context,
    event: &poise::serenity_prelude::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    tracing::info!(
        "Got an event in event handler: {:?}",
        event.snake_case_name()
    );

    if let poise::serenity_prelude::FullEvent::GuildMemberUpdate {
        old_if_available,
        new,
        ..
    } = event
    {
        let new_event_member = match new {
            Some(new) => new,
            None => {
                tracing::info!("new member is None, returning early");
                return Ok(());
            }
        };
        let user = new_event_member.user.clone();
        let guild = new_event_member.guild_id;
        let mut member = guild.member(ctx, user.clone()).await.unwrap();
        if !user
            .has_role(ctx, new_event_member.guild_id, RoleId::new(EGG_ROLE))
            .await
            .unwrap()
        {
            if new_event_member.display_name().ends_with("egg")
                || new_event_member.display_name().ends_with("EGG")
                    && new_event_member.display_name() != "Barry"
            {
                match member.remove_role(ctx, RoleId::new(NICKNAME_LICENCE)).await {
                    Ok(_res) => tracing::info!("Removed nickname licence"),
                    Err(e) => tracing::error!("{e}"),
                }

                match member.edit(ctx, EditMember::new().nickname("Barry")).await {
                    Ok(_res) => tracing::info!("set name to Barry"),
                    Err(e) => tracing::error!("{e}"),
                };
            }
            return Ok(());
        };

        let new_nick = new_event_member.display_name();
        if !new_nick.ends_with("egg") {
            match member.remove_role(ctx, RoleId::new(EGG_ROLE)).await {
                Ok(_res) => tracing::info!("Removed egg role"),
                Err(e) => tracing::error!("{e}"),
            }

            match user
                .dm(
                    ctx,
                    poise::serenity_prelude::CreateMessage::default().content(":chicken:"),
                )
                .await
            {
                Ok(_res) => tracing::info!("Sent chicken emoji"),
                Err(e) => tracing::error!("{e}"),
            }
        }

        tracing::info!(
            "Got a member update event: {:?} -> {:?}",
            old_if_available,
            new
        );
    };

    if let poise::serenity_prelude::FullEvent::Message { new_message } = event {
        if new_message.author.bot {
            return Ok(());
        }
        if data
            .paid_channels
            .lock()
            .unwrap()
            .contains_key(&new_message.channel_id)
        {
            let price: i32 = data.paid_channels.lock().unwrap()[&new_message.channel_id];

            let balance: i32 = data.db.get_balance(new_message.author.id.get()).await?;

            if balance < price {
                new_message.delete(ctx).await?;
                new_message
                .author
                .dm(
                    ctx,
                    serenity::CreateMessage::default().content(format!(
                        "Your post was deleted due to not having enough <:jbuck:1228663982462865450> to post in {}\nYour current balance: {} <:jbuck:1228663982462865450>",
                        new_message.channel(ctx).await?,
                        balance,
                    )),
                )
                .await?;
                return Ok(());
            }

            data.db
                .subtract_balances(vec![new_message.author.id.get()], price)
                .await?;

            new_message
                .author
                .dm(
                    ctx,
                    serenity::CreateMessage::default().content(format!(
                        "You paid {} <:jbuck:1228663982462865450> for posting in {}\nYour current balance: {} <:jbuck:1228663982462865450>",
                        price,
                        new_message.channel(ctx).await?,
                        balance - price,
                    )),
                )
                .await?;

            tracing::info!("Found message in paid channel, price is {}", price);
            return Ok(());
        }
    }
    Ok(())
}
