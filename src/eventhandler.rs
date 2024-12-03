use crate::database::{self, BalanceDatabase, ConfigDatabase};
use crate::discord::{EGG_ROLE, NICKNAME_LICENCE};
use crate::{Data, Error};
use ::serenity::all::{
    EditChannel, EditMember, PermissionOverwrite, PermissionOverwriteType, Permissions, RoleId,
};
use poise::serenity_prelude as serenity;
use rand::Rng;
use serenity::Result;

pub async fn event_handler(
    ctx: &serenity::Context,
    event: &poise::serenity_prelude::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    tracing::debug!(
        "Got an event in event handler: {:?}",
        event.snake_case_name()
    );

    if let poise::serenity_prelude::FullEvent::GuildMemberUpdate {
        old_if_available,
        new,
        event,
    } = event
    {
        let new_event_member = match new {
            Some(new) => new,
            None => {
                tracing::info!("new member is None, returning early");
                return Ok(());
            }
        };
        let mut user = new_event_member.user.clone();
        let guild = new_event_member.guild_id;
        match user.refresh(ctx).await {
            Ok(_) => {}
            Err(e) => tracing::error!("Error refreshing user: {e}"),
        }

        let mut member = guild.member(ctx, user.clone()).await.unwrap();
        if !event.roles.contains(&RoleId::new(EGG_ROLE)) {
            tracing::info!("doesnt have role");
            if new_event_member
                .display_name()
                .to_lowercase()
                .ends_with("egg")
            {
                tracing::info!("ends with egg, removing nickname licence");
                match member.remove_role(ctx, RoleId::new(NICKNAME_LICENCE)).await {
                    Ok(_res) => tracing::info!("Removed nickname licence"),
                    Err(e) => tracing::error!("{e}"),
                }

                tracing::info!("Barry'd");
                match member.edit(ctx, EditMember::new().nickname("Barry")).await {
                    Ok(_res) => tracing::info!("set name to Barry"),
                    Err(e) => tracing::error!("{e}"),
                };
            }
            return Ok(());
        };

        tracing::info!("has role");
        let new_nick = new_event_member.display_name();
        if !new_nick.to_lowercase().ends_with("egg") {
            tracing::info!("doesn't end with egg, removing Egg Role");
            match member.remove_role(ctx, RoleId::new(EGG_ROLE)).await {
                Ok(_res) => tracing::info!("Removed egg role"),
                Err(e) => tracing::error!("{e}"),
            }

            tracing::info!("Removed egg role");
            match user
                .dm(
                    ctx,
                    poise::serenity_prelude::CreateMessage::default().content(":chicken:"),
                )
                .await
            {
                Ok(_res) => tracing::info!("Sent chicken emoji"),
                Err(e) => tracing::error!("Error sending chicken emoji: {e}"),
            }
        }

        tracing::debug!(
            "Got a member update event: {:?} -> {:?}",
            old_if_available,
            new
        );
        return Ok(());
    };

    if let poise::serenity_prelude::FullEvent::Message { new_message } = event {
        if new_message.author.bot {
            if new_message.author.id != data.bot_id {
                return Ok(());
            }
            if new_message.content.is_empty() {
                let components = new_message.components.clone();
                if components.is_empty() {
                    return Ok(());
                }
                let click = match new_message
                    .await_component_interaction(ctx)
                    .timeout(std::time::Duration::new(60, 0))
                    .await
                {
                    Some(a) => a,
                    None => {
                        let mut new_message = new_message.clone();
                        new_message
                            .edit(ctx, {
                                serenity::EditMessage::default()
                                    .content("No one clicked the egg in time")
                                    .components(vec![])
                            })
                            .await
                            .unwrap();
                        return Ok(());
                    }
                };
                let mut user = click.user.clone();
                let guild = click.guild_id.unwrap();
                match user.refresh(ctx).await {
                    Ok(_) => {}
                    Err(e) => tracing::error!("Error refreshing user: {e}"),
                }
                let mut member = match guild.member(ctx, user.clone()).await {
                    Ok(a) => a,
                    Err(e) => {
                        tracing::error!("Error getting user who clicked egg: {e}");
                        match click
                            .create_response(
                                ctx,
                                poise::serenity_prelude::CreateInteractionResponse::Acknowledge,
                            )
                            .await
                        {
                            Ok(_) => {}
                            Err(e) => {
                                tracing::error!("Error acknowledging click: {e}");
                            }
                        };
                        return Ok(());
                    }
                };
                let nick = member.display_name();
                let egged = crate::johnny::get_egged_name(nick);

                tracing::info!("{nick} clicked the egg");

                let mut roles = member.roles.clone();

                if !roles.contains(&RoleId::new(EGG_ROLE)) {
                    roles.push(RoleId::new(EGG_ROLE));

                    data.config.write().unwrap().just_egged = Some(user.id.get());

                    match member
                        .edit(ctx, EditMember::new().nickname(egged).roles(roles))
                        .await
                    {
                        Ok(_) => {
                            tracing::info!("Changed nickname");
                        }
                        Err(e) => {
                            tracing::error!("error updating guild member : {e}");
                        }
                    }
                } else {
                    let unegged = nick.trim_end_matches(['e', 'g', 'g']);
                    roles = roles
                        .iter()
                        .filter(|role| **role != RoleId::new(EGG_ROLE))
                        .map(|role| role.to_owned())
                        .collect();
                    match member
                        .edit(ctx, EditMember::new().nickname(unegged).roles(roles))
                        .await
                    {
                        Ok(_) => tracing::info!("Removed egg role"),
                        Err(e) => tracing::error!("error updating guild member : {e}"),
                    }
                }

                match click
                    .create_response(ctx, {
                        serenity::CreateInteractionResponse::UpdateMessage(
                            serenity::CreateInteractionResponseMessage::default()
                                .content(":egg:")
                                .components(vec![]),
                        )
                    })
                    .await
                {
                    Ok(_) => {}
                    Err(e) => tracing::error!("Error setting egg as message: {e}"),
                }
            }
        };
        if data
            .paid_channels
            .lock()
            .unwrap()
            .contains_key(&new_message.channel_id)
        {
            let price: i32 = data.paid_channels.lock().unwrap()[&new_message.channel_id];

            let balance: i32 = data.db.get_balance(new_message.author.id.get()).await?;

            if balance < price {
                match new_message.delete(ctx).await {
                    Ok(_) => {
                        tracing::debug!("deleted message in paid channel due to insufficient funds")
                    }
                    Err(e) => {
                        tracing::error!("Error deleting message in paid chennel: {e}");
                        return Err(e.into());
                    }
                };
                match new_message
                .author
                .dm(
                    ctx,
                    serenity::CreateMessage::default().content(format!(
                        "Your post was deleted due to not having enough <:jbuck:1228663982462865450> to post in {}\nYour current balance: {} <:jbuck:1228663982462865450>",
                        new_message.channel(ctx).await?,
                        balance,
                    )),
                )
                .await {
                    Ok(_) => tracing::debug!("sent message deletion dm"),
                    Err(e) => tracing::error!("Error sending deletion dm: {e}"),
                };
                return Ok(());
            }

            data.db
                .subtract_balances(vec![new_message.author.id.get()], price)
                .await?;

            match new_message
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
                .await{
                    Ok(_) => tracing::debug!("Sent paid message payment dm"),
                    Err(e) => {
                        tracing::error!("Error dm'ing user for paid message: {e}");
                        return Err(e.into());
                }
                };

            tracing::info!("Found message in paid channel, price is {}", price);
            return Ok(());
        } else if data.config.read().unwrap().ghost_channel_id.is_some()
            && (data.config.read().unwrap().ghost_channel_id.unwrap()
                == new_message.channel_id.get())
        {
            let (odds, length) = {
                let config = data.config.read().unwrap();
                (
                    config.ghost_channel_odds.unwrap(),
                    config.ghost_channel_length.unwrap(),
                )
            };
            if rand::thread_rng().gen_bool(odds as f64 / 100.0) {
                println!("yap");
                let role = new_message.guild_id.unwrap().everyone_role();
                match new_message
                    .channel_id
                    .edit(
                        ctx,
                        EditChannel::new().permissions(vec![PermissionOverwrite {
                            allow: Permissions::empty(),
                            deny: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES,
                            kind: PermissionOverwriteType::Role(role),
                        }]),
                    )
                    .await
                {
                    Ok(_) => println!("Channel was privated"),
                    Err(e) => {
                        dbg!("Error privating channel", e);
                    }
                }
                {
                    let deadline = chrono::Utc::now() + chrono::Duration::minutes(length.into());
                    data.db
                        .set_config_value(
                            database::ConfigKey::UnghostTime,
                            &deadline.timestamp().to_string(),
                        )
                        .await
                        .unwrap();
                    data.config.write().unwrap().unghost_time = Some(deadline)
                }
            }
        }
    }
    Ok(())
}
