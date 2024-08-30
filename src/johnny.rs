use chrono::{Datelike, NaiveTime, TimeDelta, Timelike};
use rand::seq::SliceRandom;
use rand::Rng;
use serenity::all::{
    CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage, EditMember,
    EditMessage,
};
use serenity::gateway::ShardRunnerInfo;
use serenity::model::id::ShardId;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::Mutex;

use poise::serenity_prelude::RoleId;

use crate::database::ConfigKey;
use crate::discord::EGG_ROLE;
use crate::{
    database::{self, BalanceDatabase, ConfigDatabase, LotteryDatabase},
    game, Config, RoleDatabase,
};

type RolePrice = (i32, Option<RoleId>);
type RolePriceConfig = HashMap<RoleId, RolePrice>;

#[derive(Debug)]
pub struct Johnny {
    db: database::Database,
    price_config: Arc<RwLock<RolePriceConfig>>,
    config: Arc<RwLock<Config>>,
    channel: poise::serenity_prelude::ChannelId,
    message_client: Option<Arc<poise::serenity_prelude::Http>>,
    dev_env: bool,
    shards: Arc<Mutex<HashMap<ShardId, ShardRunnerInfo>>>,
    egg_channels: Vec<poise::serenity_prelude::ChannelId>,
}

impl Johnny {
    pub fn new(
        db: database::Database,
        t: Arc<RwLock<RolePriceConfig>>,
        config: Arc<RwLock<Config>>,
        channel: poise::serenity_prelude::ChannelId,
        client: &serenity::Client,
        dev_env: bool,
    ) -> Self {
        let a = client.shard_manager.runners.clone();
        Self {
            db,
            config,
            price_config: t,
            channel,
            message_client: Some(client.http.clone()),
            dev_env,
            shards: a,
            egg_channels: vec![
                poise::serenity_prelude::ChannelId::from(1128350001328816343),
                poise::serenity_prelude::ChannelId::from(1224695899796541554),
            ],
        }
    }
    pub async fn start(&self, signal: std::sync::mpsc::Receiver<()>) {
        let mut minute_counter = tokio::time::Instant::now();
        let mut five_minute_counter = tokio::time::Instant::now();
        let mut last_lottery: Option<chrono::NaiveDateTime> = None;
        self.refresh_config().await;
        loop {
            match signal.try_recv() {
                Ok(_) => {
                    tracing::debug!("Johnny received signal to stop");
                    break;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
                Err(e) => {
                    tracing::error!("{}", e.to_string());
                    break;
                }
            }
            if self.should_trigger_lottery(last_lottery).await {
                last_lottery = Some(chrono::Utc::now().naive_utc());
                self.lottery().await;
            }

            if self.should_update_skewed_odds().await {
                self.update_skewed_odds().await;
            }

            let force_egg = { self.config.read().unwrap().force_egg };
            if self.should_run_egg(force_egg).await {
                self.config.write().unwrap().force_egg = false;
                self.db
                    .set_config_value(ConfigKey::ForceEgg, "false")
                    .await
                    .unwrap();
                tracing::info!("running egg");
                self.run_egg().await;
            }

            if minute_counter.elapsed().as_secs() >= 60 {
                self.refresh_config().await;
                let (bones_price_updated, force) = {
                    let c = self.config.read().unwrap();
                    (c.bones_price_updated, c.bones_price_force_update)
                };
                if self.should_update_bones_price(bones_price_updated) || force {
                    self.update_bones_price().await;
                }
                if force {
                    tracing::info!("toggling bones price force");
                    self.db
                        .set_config_value(ConfigKey::ForceBonesPriceUpdate, "false")
                        .await
                        .unwrap();
                    self.config.write().unwrap().bones_price_force_update = false;
                }
                if self.should_decay_bones() {
                    self.decay_bones().await;
                }

                minute_counter = tokio::time::Instant::now();
            }

            if five_minute_counter.elapsed().as_secs() >= 300 {
                self.decay().await;
                five_minute_counter = tokio::time::Instant::now();
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }

    pub async fn decay(&self) {
        let data = {
            match self.db.get_price_decay_config().await {
                Ok(result) => result,
                Err(e) => {
                    tracing::error!(e);
                    vec![]
                }
            }
        };

        let now = chrono::Utc::now();
        for config in data {
            let last = config.last_decay;
            if last.checked_add_signed(TimeDelta::hours(config.interval.into())) < Some(now) {
                match self
                    .db
                    .decay_role_price(config.role_id, config.amount, config.minimum)
                    .await
                {
                    Ok(r) => {
                        let parsed = poise::serenity_prelude::RoleId::new(r.role_id);
                        let mut config = self.price_config.write().unwrap();
                        config.get_mut(&parsed).unwrap().0 = r.price;
                    }
                    Err(e) => {
                        tracing::error!(e);
                    }
                }
                match self.db.price_decayed(config.role_id).await {
                    Ok(_) => {}
                    Err(e) => {
                        tracing::error!(e);
                    }
                }
            }
        }
    }

    pub async fn refresh_config(&self) {
        match self.db.get_config().await {
            Ok(r) => match self.config.write() {
                Ok(mut c) => {
                    let counter = c.bot_odds_game_counter;
                    *c = Config::from(r);
                    c.bot_odds_game_counter = counter;
                }
                Err(e) => {
                    tracing::error!("{e}");
                }
            },
            Err(e) => {
                tracing::error!(e);
            }
        }
    }

    async fn should_update_skewed_odds(&self) -> bool {
        let (limit, counter) = {
            let config = match self.config.read() {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("{e}");
                    return false;
                }
            };
            (config.bot_odds_game_limit, config.bot_odds_game_counter)
        };
        tracing::debug!(limit, counter);

        counter > limit
    }

    pub async fn update_skewed_odds(&self) {
        let bot_odds = rand::thread_rng().gen_range(0.3..=0.7);
        self.db
            .set_config_value(database::ConfigKey::BotOdds, &bot_odds.to_string())
            .await
            .unwrap();
        self.db
            .set_config_value(
                database::ConfigKey::BotOddsUpdated,
                &chrono::Utc::now().timestamp().to_string(),
            )
            .await
            .unwrap();

        let mut c = match self.config.write() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("{e}");
                return;
            }
        };
        c.bot_odds_updated = Some(chrono::Utc::now());
        c.bot_odds = bot_odds;
        c.bot_odds_game_counter = 0;
    }

    pub async fn should_trigger_lottery(&self, last: Option<chrono::NaiveDateTime>) -> bool {
        let time = chrono::Utc::now()
            .time()
            .with_second(0)
            .unwrap()
            .with_nanosecond(0)
            .unwrap();

        let today = chrono::Utc::now().date_naive();
        let time = today.and_time(time).time();

        if (time == NaiveTime::from_hms_opt(18, 00, 0).unwrap()
            && (last.is_none() || last.unwrap().date() != today))
            || self.dev_env
        {
            return true;
        }
        false
    }

    pub async fn lottery(&self) {
        let (base_prize, price) = {
            let config = self.config.read().unwrap();
            (config.lottery_base_prize, config.lottery_ticket_price)
        };
        let lottery_tickets = self.db.get_bought_tickets().await.unwrap();
        let pot = lottery_tickets
            .iter()
            .map(|(_, x)| x * (price - 1))
            .sum::<i32>()
            + base_prize;
        let lottery = game::Lottery::new(lottery_tickets.clone());
        let winner = match self.config.read().unwrap().lottery_winner {
            Some(x) => x,
            None => lottery.get_winner(),
        };

        if winner == 0 {
            return;
        }

        self.db.award_balances(vec![winner], pot).await.unwrap();
        let (new_base_prize, new_ticket_price) = {
            let config = self.config.read().unwrap();
            (
                config.future_lottery_base_prize,
                config.future_lottery_ticket_price,
            )
        };
        {
            let mut config = self.config.write().unwrap();
            config.lottery_base_prize = new_base_prize;
            config.lottery_ticket_price = new_ticket_price;
        }
        self.db
            .set_config_value(
                database::ConfigKey::LotteryTicketPrice,
                &new_ticket_price.to_string(),
            )
            .await
            .unwrap();
        self.db
            .set_config_value(
                database::ConfigKey::LotteryBasePrize,
                &new_base_prize.to_string(),
            )
            .await
            .unwrap();

        self.db
            .del_config_value(database::ConfigKey::LotteryWinner)
            .await
            .unwrap();
        self.config.write().unwrap().lottery_winner = None;

        let losers = lottery_tickets
            .iter()
            .map(|(a, _)| a)
            .filter(|a| **a != winner)
            .collect::<Vec<_>>();

        let loser_text = if losers.is_empty() {
            "".to_string()
        } else {
            let text = losers
                .iter()
                .map(|a| format!("<@{}>", a))
                .collect::<Vec<String>>()
                .join(", ");
            format!("> Losers: {}\n", text)
        };

        let num_tickets = match lottery_tickets.iter().find(|a| a.0 == winner) {
            Some(a) => a.1,
            None => 0,
        };
        let text = format!("> :tada: :tada: WOW! <@{}> just won the lottery!\n> They won **{} <:jbuck:1228663982462865450>** by buying only **{} :tickets:**\n{}> \n> **New lottery starting... NOW**\n> Prize pool: {} <:jbuck:1228663982462865450>\n> Use ***/lottery buy*** to purchase a ticket for {} <:jbuck:1228663982462865450>",
            winner, pot, num_tickets, loser_text, new_base_prize, new_ticket_price);

        let m = { CreateMessage::new().content(text) };

        if let Some(client) = &self.message_client {
            self.channel.send_message(client, m).await.unwrap();
        } else {
            tracing::warn!("Discord client not set");
        }

        self.db.clear_tickets().await.unwrap();
    }

    fn should_update_bones_price(&self, last: chrono::DateTime<chrono::Utc>) -> bool {
        let time = chrono::Utc::now()
            .time()
            .with_second(0)
            .unwrap()
            .with_nanosecond(0)
            .unwrap();

        if (time == NaiveTime::from_hms_opt(0, 0, 0).unwrap()
            || time == NaiveTime::from_hms_opt(12, 0, 0).unwrap())
            && last.checked_add_signed(TimeDelta::minutes(55)).unwrap() < chrono::Utc::now()
        {
            return true;
        }
        if last.checked_add_signed(TimeDelta::minutes(5)).unwrap() < chrono::Utc::now()
            && self.dev_env
        {
            return true;
        }
        false
    }

    async fn update_bones_price(&self) {
        let (min, max, old_price, last_was_increase) = {
            let config = self.config.read().unwrap();
            let a = if is_weekend() {
                (25, None)
            } else {
                (config.bones_price, config.bones_price_last_was_increase)
            };
            (config.bones_price_min, config.bones_price_max, a.0, a.1)
        };
        if min > max {
            tracing::error!("Invalid bones price range. Min: {}, Max: {}", min, max);
            return;
        }
        let mut change = min;
        if min < max {
            change = rand::thread_rng().gen_range(min..=max);
        }

        let odds: f64;
        if last_was_increase.is_none() {
            odds = 0.5;
        } else if last_was_increase.unwrap() {
            odds = 0.6;
        } else {
            odds = 0.4;
        };
        let mut price: i32 = if rand::thread_rng().gen_bool(odds) {
            old_price + change
        } else {
            old_price - change
        };
        if price < 0 {
            price = 0;
        }

        self.db
            .set_config_value(ConfigKey::BonesPrice, &price.to_string())
            .await
            .unwrap();
        self.db
            .set_config_value(
                ConfigKey::BonesPriceUpdated,
                &chrono::Utc::now().timestamp().to_string(),
            )
            .await
            .unwrap();
        self.db
            .set_config_value(
                ConfigKey::BonesPriceLastWasIncrease,
                &(price > old_price).to_string(),
            )
            .await
            .unwrap();
        {
            let mut config = self.config.write().unwrap();
            config.bones_price = price;
            config.bones_price_updated = chrono::Utc::now();
            config.bones_price_last_was_increase = Some(price > old_price);
        }

        let m = {
            CreateMessage::new().content(format!(
                ":bone: I just set the bones price to {} <:jbuck:1228663982462865450>",
                price
            ))
        };

        if let Some(client) = &self.message_client {
            self.channel.send_message(client, m).await.unwrap();
        } else {
            tracing::warn!("Discord client not set");
        }
    }

    fn should_decay_bones(&self) -> bool {
        let time = chrono::Utc::now()
            .time()
            .with_second(0)
            .unwrap()
            .with_nanosecond(0)
            .unwrap();
        if time == NaiveTime::from_hms_opt(0, 0, 0).unwrap()
            && chrono::Utc::now().weekday() == chrono::Weekday::Sat
        {
            return true;
        }
        false
    }

    async fn decay_bones(&self) {
        let affected = self.db.decay_bones().await.unwrap();
        let m = {
            CreateMessage::new().content(format!(
                ":bone: I just decayed all bones from the economy! :bone:\n{} people lost all their bones",
                affected.len()
            ))
        };

        if let Some(client) = &self.message_client {
            self.channel.send_message(client, m).await.unwrap();
            for person in affected {
                let u = poise::serenity_prelude::UserId::new(person);
                u.dm(
                    client,
                    poise::serenity_prelude::CreateMessage::default()
                        .content("Oh no, your bones expired!"),
                )
                .await
                .unwrap();
            }
        } else {
            tracing::warn!("Discord client not set");
        }
    }

    async fn should_run_egg(&self, force: bool) -> bool {
        rand::thread_rng().gen_bool(1.0 / 604800.0) || force
    }

    async fn run_egg(&self) {
        let m = {
            CreateMessage::new().content(":egg:").components(vec![
                poise::serenity_prelude::CreateActionRow::Buttons(vec![
                    poise::serenity_prelude::CreateButton::new("accept_egg").label("Yes"),
                ]),
            ])
        };
        if let Some(client) = &self.message_client {
            let channel = { self.egg_channels.choose(&mut rand::thread_rng()).unwrap() };
            let mut sent = channel.send_message(client, m).await.unwrap();
            let click = match sent
                .await_component_interaction(
                    self.shards.try_lock().unwrap().values().nth(0).unwrap(),
                )
                .timeout(std::time::Duration::new(60, 0))
                .await
            {
                Some(a) => a,
                None => {
                    sent.edit(client, {
                        EditMessage::default()
                            .content("No one clicked the egg in time")
                            .components(vec![])
                    })
                    .await
                    .unwrap();
                    return;
                }
            };
            let user = click.user.clone();
            let guild = click.guild_id.unwrap();
            let mut member = guild.member(client, user.clone()).await.unwrap();
            let nick = member.display_name();
            let egged = get_egged_name(nick);
            match member.add_role(client, RoleId::new(EGG_ROLE)).await {
                Ok(_) => {
                    tracing::info!("Assigned egg role");
                }
                Err(e) => {
                    tracing::error!("{e}");
                }
            }

            match member.edit(client, EditMember::new().nickname(egged)).await {
                Ok(_) => {
                    tracing::info!("Changed nickname");
                }
                Err(e) => {
                    tracing::error!("{e}");
                }
            }

            match click
                .create_response(client, {
                    CreateInteractionResponse::UpdateMessage(
                        CreateInteractionResponseMessage::default()
                            .content(format!("{} has been egged", click.user))
                            .components(vec![]),
                    )
                })
                .await
            {
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("{e}");
                }
            }
        } else {
            tracing::warn!("Discord client not set");
        }
    }
}

fn get_egged_name(nick: &str) -> String {
    // name has to be max 32 characters
    if nick.len() > 29 {
        let mut res = nick.chars().take(29).collect::<String>();
        res.push_str("egg");
        return res;
    }

    // find the last vowel
    let vowels = ['a', 'e', 'i', 'o', 'u'];
    let mut last_vowel = None;
    for (i, c) in nick.chars().enumerate() {
        if vowels.contains(&c) {
            last_vowel = Some(i);
        }
    }
    // replace from the last vowel to the end with "egg"
    match last_vowel {
        Some(i) => {
            let mut new = nick.chars().take(i).collect::<String>();
            new.push_str("egg");
            new
        }
        None => format!("{}egg", nick),
    }
}

pub fn is_weekend() -> bool {
    let now = chrono::Utc::now();
    now.weekday() == chrono::Weekday::Sat || now.weekday() == chrono::Weekday::Sun
}
