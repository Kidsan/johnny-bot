use crate::{database::BalanceDatabase, Context, Error};
use poise::CreateReply;

///
/// Buy something with your JBucks
///
/// Enter `/buy `
/// ```
/// /buy role @role
/// ```
#[poise::command(slash_command, subcommands("role"), subcommand_required)]
pub async fn buy(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

///
/// Buy a role with your JBucks
///
/// Enter `/buy role @role`
/// ```
/// /buy role @JohnnyBot
/// ```
#[poise::command(slash_command)]
pub async fn role(
    ctx: Context<'_>,
    #[description = "role to purchase"] role: poise::serenity_prelude::Role,
) -> Result<(), Error> {
    if !ctx.data().roles.lock().unwrap().contains_key(&role.id) {
        let reply = {
            CreateReply::default()
                .content("That role is not for sale!")
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("Role not for sale".into());
    }

    // check if user has the role already
    if ctx
        .author()
        .has_role(ctx, ctx.guild_id().unwrap(), role.id)
        .await?
    {
        let reply = {
            CreateReply::default()
                .content("You already have that role!")
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("Role already owned".into());
    }

    let balance = ctx
        .data()
        .db
        .get_balance(ctx.author().id.to_string())
        .await?;

    let price = { ctx.data().roles.lock().unwrap()[&role.id] };

    if balance < price {
        let reply = {
            CreateReply::default()
                .content(format!(
                    "You can't afford that role! You need {} <:jbuck:1228663982462865450>!",
                    price
                ))
                .ephemeral(true)
        };
        ctx.send(reply).await?;
        return Err("Not enough money".into());
    }

    ctx.data()
        .db
        .subtract_balances(vec![ctx.author().id.to_string()], price)
        .await?;

    let remove = {
        ctx.data().unique_roles.contains(&role.id)
        // find users with the role

        // ctx.serenity_context()
        //     .http
        //     .remove_member_roles(ctx.guild_id().unwrap(), ctx.author().id, &roles)
        //     .await?;
    };
    if remove {
        // remove the role from anyone who has it
    }

    // give the user the role
    ctx.serenity_context()
        .http
        .add_member_role(ctx.guild_id().unwrap(), ctx.author().id, role.id, None)
        .await?;

    ctx.say(format!("You want to buy a role {}", role)).await?;
    Ok(())
}
