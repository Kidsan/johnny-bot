use crate::database::BalanceDatabase;
use crate::{Data, Error};
use poise::serenity_prelude as serenity;
use serenity::Result;

pub async fn event_handler(
    ctx: &serenity::Context,
    event: &poise::serenity_prelude::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    println!(
        "Got an event in event handler: {:?}",
        event.snake_case_name()
    );

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

            let balance: i32 = data
                .db
                .get_balance(new_message.author.id.get().try_into().unwrap())
                .await?;

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
                .subtract_balances(vec![new_message.author.id.to_string()], price)
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

            println!("Found message in paid channel, price is {}", price);
            return Ok(());
        }
    }
    Ok(())
}
