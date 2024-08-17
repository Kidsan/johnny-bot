use chrono::{Datelike, NaiveTime, TimeDelta, Timelike};
use rand::Rng;
use serenity::all::CreateMessage;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use poise::serenity_prelude::RoleId;

use crate::database::ConfigKey;
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
    client: Option<Arc<poise::serenity_prelude::Http>>,
    dev_env: bool,
}

impl Johnny {
    pub fn new(
        db: database::Database,
        t: Arc<RwLock<RolePriceConfig>>,
        config: Arc<RwLock<Config>>,
        channel: poise::serenity_prelude::ChannelId,
        client: Option<Arc<poise::serenity_prelude::Http>>,
        dev_env: bool,
    ) -> Self {
        Self {
            db,
            config,
            price_config: t,
            channel,
            client,
            dev_env,
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
                    dbg!("Johnny received signal to stop");
                    break;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
                Err(e) => {
                    dbg!(e);
                    break;
                }
            }
            if self.should_trigger_lottery(last_lottery).await {
                last_lottery = Some(chrono::Utc::now().naive_utc());
                self.lottery().await;
            }

            if minute_counter.elapsed().as_secs() >= 60 {
                let _ = rand::thread_rng().gen_range(0..=100);
                self.refresh_config().await;
                let (bones_price_updated, force) = {
                    let c = self.config.read().unwrap();
                    (c.bones_price_updated, c.bones_price_force_update)
                };
                if self.should_update_bones_price(bones_price_updated) || force {
                    self.update_bones_price().await;
                }
                if force {
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
                if self.should_update_skewed_odds().await {
                    self.update_skewed_odds().await;
                }
                five_minute_counter = tokio::time::Instant::now();
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    }

    pub async fn decay(&self) {
        let data = {
            match self.db.get_price_decay_config().await {
                Ok(result) => result,
                Err(e) => {
                    dbg!(e);
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
                        dbg!(e);
                    }
                }
                match self.db.price_decayed(config.role_id).await {
                    Ok(_) => {}
                    Err(e) => {
                        dbg!(e);
                    }
                }
            }
        }
    }

    pub async fn refresh_config(&self) {
        match self.db.get_config().await {
            Ok(r) => *self.config.write().unwrap() = Config::from(r),
            Err(e) => {
                dbg!(e);
            }
        }
    }

    async fn should_update_skewed_odds(&self) -> bool {
        let last = self.db.get_config().await.unwrap().bot_odds_updated;

        if last.is_none() {
            return true;
        }

        if chrono::Utc::now().timestamp() - last.unwrap().timestamp() >= 24 * 60 * 60 {
            return true;
        }
        false
    }

    pub async fn update_skewed_odds(&self) {
        let bot_odds = rand::thread_rng().gen_range(0.2..=0.7);
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

        self.config.write().unwrap().bot_odds = bot_odds;
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
        let winner = lottery.get_winner();

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

        let num_tickets = lottery_tickets.iter().find(|a| a.0 == winner).unwrap().1;
        let text = format!("> :tada: :tada: WOW! <@{}> just won the lottery!\n> They won **{} <:jbuck:1228663982462865450>** by buying only **{} :tickets:**\n{}> \n> **New lottery starting... NOW**\n> Prize pool: {} <:jbuck:1228663982462865450>\n> Use ***/lottery buy*** to purchase a ticket for {} <:jbuck:1228663982462865450>",
            winner, pot, num_tickets, loser_text, new_base_prize, new_ticket_price);

        let m = { CreateMessage::new().content(text) };

        if let Some(client) = &self.client {
            self.channel.send_message(client, m).await.unwrap();
        } else {
            dbg!("Client not set");
        }

        self.db.clear_tickets().await.unwrap();
    }

    fn should_update_bones_price(&self, last: chrono::DateTime<chrono::Utc>) -> bool {
        let time = chrono::Utc::now()
            .time()
            .with_second(0)
            .unwrap()
            .with_nanosecond(0)
            .unwrap()
            .with_minute(0)
            .unwrap();

        if (time == NaiveTime::from_hms_opt(0, 0, 0).unwrap()
            || time == NaiveTime::from_hms_opt(12, 0, 0).unwrap())
            && last.checked_add_signed(TimeDelta::minutes(2)).unwrap() < chrono::Utc::now()
        {
            dbg!("updating bones price ");
            return true;
        }
        if last.checked_add_signed(TimeDelta::minutes(5)).unwrap() < chrono::Utc::now()
            && self.dev_env
        {
            dbg!("updating bones price for dev_env setting");
            return true;
        }
        dbg!("Not updating bones price");
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

        if let Some(client) = &self.client {
            self.channel.send_message(client, m).await.unwrap();
        } else {
            dbg!("Client not set");
        }
    }

    fn should_decay_bones(&self) -> bool {
        let time = chrono::Utc::now()
            .time()
            .with_second(0)
            .unwrap()
            .with_minute(0)
            .unwrap()
            .with_nanosecond(0)
            .unwrap();
        if time == NaiveTime::from_hms_opt(0, 0, 0).unwrap()
            && chrono::Utc::now().weekday() == chrono::Weekday::Sat
        {
            dbg!("decaying bones");
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

        if let Some(client) = &self.client {
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
            dbg!("Client not set");
        }
    }
}

pub fn is_weekend() -> bool {
    let now = chrono::Utc::now();
    now.weekday() == chrono::Weekday::Sat || now.weekday() == chrono::Weekday::Sun
}
