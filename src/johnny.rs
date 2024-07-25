use std::sync::{Arc, RwLock};

use chrono::{NaiveTime, TimeDelta, Timelike};
use rand::Rng;
use serenity::all::CreateMessage;
use std::collections::HashMap;

use poise::serenity_prelude::RoleId;

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
        let mut time_passed = 0;
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

            if time_passed % 60 == 0 {
                let _ = rand::thread_rng().gen_range(0..=100);
                self.check_should_trigger_lottery().await;
                self.refresh_config().await;
            }

            if time_passed % 300 == 0 {
                self.decay().await;
                self.check_skewed_odds().await;
                time_passed = 0;
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            time_passed += 5;
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
            Ok(r) => {
                self.config.write().unwrap().daily_upper_limit = r.daily_upper_limit.unwrap_or(0);
                self.config.write().unwrap().bot_odds = r.bot_odds.unwrap_or(0.5);
            }
            Err(e) => {
                dbg!(e);
            }
        }
    }

    async fn check_skewed_odds(&self) {
        let today = chrono::Utc::now()
            .with_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
            .unwrap();

        if let Some(last_updated) = self.db.get_config().await.unwrap().bot_odds_updated {
            if last_updated < today {
                self.update_skewed_odds().await;
                return;
            }
            return;
        }
        self.update_skewed_odds().await;
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

    pub async fn check_should_trigger_lottery(&self) {
        let time = chrono::Utc::now()
            .time()
            .with_second(0)
            .unwrap()
            .with_nanosecond(0)
            .unwrap();
        if time == NaiveTime::from_hms_opt(18, 00, 0).unwrap() || self.dev_env {
            self.lottery().await;
        }
    }

    pub async fn lottery(&self) {
        let lottery_tickets = self.db.get_bought_tickets().await.unwrap();
        let pot = lottery_tickets.iter().map(|(_, x)| x * 5).sum::<i32>() + 10;
        let lottery = game::Lottery::new(lottery_tickets.clone(), pot);
        let winner = lottery.get_winner();

        if winner == 0 {
            return;
        }

        self.db.award_balances(vec![winner], pot).await.unwrap();

        let num_tickets = lottery_tickets.iter().find(|a| a.0 == winner).unwrap().1;
        let text = format!("> :tada: :tada: WOW! <@{}> just won the lottery!\n> They won **{} <:jbuck:1228663982462865450>** by buying only **{} :tickets:**\n> \n> **New lottery starting... NOW**\n> Prize pool: {} <:jbuck:1228663982462865450>\n> Use ***/lottery buy*** to purchase a ticket for 5 <:jbuck:1228663982462865450>", winner, pot, num_tickets
            , 10);

        let m = { CreateMessage::new().content(text) };

        if let Some(client) = &self.client {
            self.channel.send_message(client, m).await.unwrap();
        } else {
            dbg!("Client not set");
        }

        self.db.clear_tickets().await.unwrap();
    }
}
