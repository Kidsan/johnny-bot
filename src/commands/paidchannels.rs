use crate::{database::ChannelDatabase, Context, Error};
use poise::CreateReply;

///
/// Set a price on a channel
///
/// Enter `/setchannelprice <amount>` to set the price
/// ```
/// /setchannelprice 10
/// ```
#[poise::command(
    slash_command,
    category = "Admin",
    default_member_permissions = "ADMINISTRATOR",
    hide_in_help
)]
pub async fn setchannelprice(
    ctx: Context<'_>,
    #[description = "Amount of J-Bucks to send a message in this channel"]
    #[min = 0]
    amount: i32,
) -> Result<(), Error> {
    let c = ctx.channel_id().get();
    if amount == 0 {
        ctx.data()
            .paid_channels
            .lock()
            .unwrap()
            .remove(&ctx.channel_id());
        ctx.data().db.remove_paid_channel(c).await?;
    } else {
        ctx.data()
            .paid_channels
            .lock()
            .unwrap()
            .insert(ctx.channel_id(), amount);
        ctx.data().db.set_channel_price(c, amount).await?;
    }
    ctx.send(
        CreateReply::default()
            .ephemeral(true)
            .content(format!("Channel price set to {}", amount)),
    )
    .await?;
    Ok(())
}
