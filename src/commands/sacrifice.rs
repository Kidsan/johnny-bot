use crate::database::BalanceDatabase;
use crate::{Context, Error};

#[derive(Debug, poise::ChoiceParameter, Clone)]
pub enum SacrificeReasons {
    Robbery,
}

///
/// sacrifice some bones for rewards
///
/// Enter `/sacrifice <reason>`
/// ```
/// /sacrifice robbery
/// ```
#[poise::command(slash_command)]
#[tracing::instrument(level = "info")]
pub async fn sacrifice(ctx: Context<'_>, reason: SacrificeReasons) -> Result<(), Error> {
    match reason {
        SacrificeReasons::Robbery => {
            let _balance = ctx.data().db.get_bones(ctx.author().id.get()).await?;
        }
    };
    Ok(())
}

// ///
// /// sacrifice some bones to enable robbing again
// ///
// /// ```
// /// /sacrifice bones <reason>
// /// ```
// #[poise::command(
//     slash_command,
// )]
// pub async fn for_robbery(ctx: Context<'_>) -> Result<(), Error> {
//
// }
