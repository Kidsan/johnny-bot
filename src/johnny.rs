use std::sync::{Arc, RwLock};

use chrono::{NaiveTime, TimeDelta};
use rand::Rng;
use std::collections::HashMap;

use poise::serenity_prelude::RoleId;

use crate::{
    database::{self, ConfigDatabase},
    Config, RoleDatabase,
};

type RolePrice = (i32, Option<RoleId>);
type RolePriceConfig = HashMap<RoleId, RolePrice>;

#[derive(Debug)]
pub struct Johnny {
    db: database::Database,
    price_config: Arc<RwLock<RolePriceConfig>>,
    config: Arc<RwLock<Config>>,
}

impl Johnny {
    pub fn new(
        db: database::Database,
        t: Arc<RwLock<RolePriceConfig>>,
        config: Arc<RwLock<Config>>,
    ) -> Self {
        Self {
            db,
            config,
            price_config: t,
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
}
