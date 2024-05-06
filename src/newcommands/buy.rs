use crate::{database::BalanceDatabase, Context, Error};
use poise::CreateReply;

#[poise::command(slash_command)]
pub async fn shop(ctx: Context<'_>) -> Result<(), Error> {
    let reply = {
        let roles = ctx.data().roles.lock().unwrap();
        let mut role_prices = roles
            .iter()
            .map(|(role_id, price)| {
                format!("> <@&{}> - {} <:jbuck:1228663982462865450>", role_id, price)
            })
            .collect::<Vec<String>>()
            .join("\n");
        role_prices.insert_str(0, "**Roles for sale:**\n");
        role_prices.insert_str(
            0,
            "### <:jbuck:1228663982462865450> Shop <:jbuck:1228663982462865450> ###\n\n",
        );
        CreateReply::default().content(role_prices).ephemeral(true)
    };
    ctx.send(reply).await?;

    Ok(())
}

#[poise::command(
    slash_command,
    category = "Admin",
    default_member_permissions = "ADMINISTRATOR",
    hide_in_help
)]
pub async fn setroleprice(
    ctx: Context<'_>,
    price: i32,
    role: poise::serenity_prelude::Role,
) -> Result<(), Error> {
    ctx.data()
        .db
        .set_role_price(role.id.to_string().parse()?, price)
        .await?;
    ctx.data().roles.lock().unwrap().insert(role.id, price);
    let reply = {
        CreateReply::default()
            .content(format!(
                "You have set the price for the role {} to {}!",
                role, price
            ))
            .ephemeral(true)
    };
    ctx.send(reply).await?;
    Ok(())
}

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

    // give the user the role
    ctx.serenity_context()
        .http
        .add_member_role(
            ctx.guild_id().unwrap(),
            ctx.author().id,
            role.id,
            Some("Buying a role"),
        )
        .await?;

    let reply = {
        CreateReply::default()
            .content(format!(
                "You have purchased the role {} for {} <:jbuck:1228663982462865450>!",
                role, price
            ))
            .ephemeral(true)
    };
    ctx.send(reply).await?;

    Ok(())
}
