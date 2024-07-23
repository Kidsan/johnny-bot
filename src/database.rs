use chrono::{DateTime, Utc};
use sqlx::Pool;

#[cfg(not(test))]
use tokio::fs;

use crate::Error;

#[derive(Debug, sqlx::FromRow)]
struct Balance {
    balance: i32,
}

#[derive(Debug, sqlx::FromRow)]
struct Daily {
    last_daily: sqlx::types::chrono::DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
struct Total {
    total: i32,
}

#[derive(Debug, sqlx::FromRow)]
struct Average {
    average: f32,
}

#[derive(Debug, sqlx::FromRow)]
struct BoughtRobbery {
    last_bought: i64,
}

#[derive(Debug, sqlx::FromRow)]
pub struct PurchaseableRoleConfig {
    pub role_id: i64,
    pub price: i32,
    pub only_one: bool,
    pub increment: Option<i32>,
    pub required_role_id: Option<i64>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct PurchaseableRole {
    pub role_id: u64,
    pub price: i32,
    pub only_one: bool,
    pub increment: Option<i32>,
    pub required_role_id: Option<u64>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct RoleHolder {
    pub user_id: u64,
    pub purchased: sqlx::types::chrono::DateTime<Utc>,
}

#[allow(async_fn_in_trait)]
pub trait BalanceDatabase {
    async fn get_balance(&self, user_id: u64) -> Result<i32, Error>;
    async fn award_balances(&self, user_ids: Vec<u64>, award: i32) -> Result<(), Error>;
    async fn subtract_balances(&self, user_ids: Vec<u64>, amount: i32) -> Result<(), Error>;
    async fn get_leaderboard(&self) -> Result<Vec<(u64, i32)>, Error>;
    async fn get_last_daily(&self, user_id: u64) -> Result<Option<DateTime<Utc>>, Error>;
    async fn did_daily(&self, user_id: u64) -> Result<(), Error>;
    async fn get_total(&self) -> Result<i32, Error>;
    async fn get_avg_balance(&self) -> Result<f32, Error>;
    async fn get_zero_balance(&self) -> Result<i32, Error>;
    async fn bury_balance(&self, user_id: u64, amount: i32) -> Result<(), Error>;
    async fn get_dailies_today(&self) -> Result<i32, Error>;
    async fn get_crown_leaderboard(&self) -> Result<Vec<(u64, f32)>, Error>;
    async fn update_crown_timer(&self, user_id: u64, hours: f32) -> Result<(), Error>;
}

pub trait RobberyDatabase {
    async fn get_last_bought_robbery(&self, user_id: u64) -> Result<DateTime<Utc>, Error>;
    async fn bought_robbery(&self, user_id: u64) -> Result<(), Error>;
}

pub trait ChannelDatabase {
    async fn get_paid_channels(&self) -> Result<Vec<(u64, i32)>, Error>;
    async fn set_channel_price(&self, channel_id: u64, price: i32) -> Result<(), Error>;
    async fn remove_paid_channel(&self, channel_id: u64) -> Result<(), Error>;
}

pub trait RoleDatabase {
    async fn price_decayed(&self, role_id: u64) -> Result<(), Error>;
    async fn get_purchasable_roles(&self) -> Result<Vec<PurchaseableRole>, Error>;
    async fn increment_role_price(&self, role_id: String) -> Result<(), Error>;
    async fn set_role_price(
        &self,
        role_id: u64,
        price: i32,
        increment: Option<i32>,
        required_role: Option<u64>,
        only_one: Option<bool>,
    ) -> Result<(), Error>;
    async fn toggle_role_unique(&self, role_id: u64, only_one: bool) -> Result<(), Error>;
    async fn get_unique_role_holder(&self, role_id: u64) -> Result<Option<RoleHolder>, Error>;
    async fn set_unique_role_holder(&self, role_id: u64, user_id: u64) -> Result<(), Error>;
    async fn get_price_decay_config(&self) -> Result<Vec<RolePriceDecayConfig>, Error>;
    async fn set_price_decay_config(
        &self,
        role_id: u64,
        amount: i32,
        interval: i32,
        minimum: i32,
    ) -> Result<(), Error>;
    async fn decay_role_price(
        &self,
        role_id: u64,
        price: i32,
        minimum: i32,
    ) -> Result<PurchaseableRole, Error>;
}

pub trait ConfigDatabase {
    async fn get_config(&self) -> Result<Config, Error>;
    async fn set_config_value(&self, key: ConfigKey, value: &str) -> Result<(), Error>;
}

#[derive(Debug, sqlx::FromRow)]
struct ConfigRow {
    pub key: String,
    pub value: String,
}

pub enum ConfigKey {
    DailyUpperLimit,
    BotOddsUpdated,
    BotOdds,
}

impl ConfigKey {
    fn as_str(&self) -> &str {
        match self {
            ConfigKey::DailyUpperLimit => "daily_upper_limit",
            ConfigKey::BotOddsUpdated => "bot_odds_updated",
            ConfigKey::BotOdds => "bot_odds",
        }
    }
}

impl ConfigDatabase for Database {
    async fn get_config(&self) -> Result<Config, Error> {
        let data = sqlx::query_as::<_, ConfigRow>("SELECT * FROM config")
            .fetch_all(&self.connection)
            .await?;

        let mut config = Config {
            daily_upper_limit: None,
            bot_odds_updated: None,
            bot_odds: None,
        };

        for d in data {
            if ConfigKey::DailyUpperLimit.as_str() == d.key.as_str() {
                config.daily_upper_limit = Some(d.value.parse().unwrap());
            }
            if ConfigKey::BotOddsUpdated.as_str() == d.key.as_str() {
                config.bot_odds_updated = Some(
                    chrono::DateTime::from_timestamp(d.value.parse().unwrap(), 0).unwrap_or(
                        chrono::Utc::now()
                            .checked_sub_days(chrono::Days::new(1))
                            .unwrap(),
                    ),
                );
            }
            if ConfigKey::BotOdds.as_str() == d.key.as_str() {
                config.bot_odds = Some(d.value.parse().unwrap());
            }
        }
        Ok(config)
    }
    async fn set_config_value(&self, key: ConfigKey, value: &str) -> Result<(), Error> {
        sqlx::query("INSERT INTO config (key, value) VALUES ($1, $2) ON CONFLICT(key) DO UPDATE SET value = $2")
            .bind(key.as_str())
            .bind(value)
            .execute(&self.connection)
            .await?;
        Ok(())
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct RolePriceDecayConfig {
    pub role_id: u64,
    pub amount: i32,
    pub interval: i32,
    pub minimum: i32,
    pub last_decay: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct RolePriceDecay {
    pub role_id: i64,
    pub amount: i32,
    pub interval: i32,
    pub minimum: i32,
    pub last_decay: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct Config {
    pub daily_upper_limit: Option<i32>,
    pub bot_odds_updated: Option<chrono::DateTime<Utc>>,
    pub bot_odds: Option<f32>,
}

#[derive(Debug)]
pub struct Database {
    pub connection: Pool<sqlx::Sqlite>,
}

impl Database {
    #[tracing::instrument(level = "info")]
    #[cfg(not(test))]
    pub async fn new() -> Result<Self, Error> {
        fs::create_dir_all("./data").await?;
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename("./data/johnny.db")
            .optimize_on_close(true, None)
            .shared_cache(false)
            .create_if_missing(true);

        let pool = sqlx::sqlite::SqlitePool::connect_with(options).await?;
        sqlx::migrate!().run(&pool).await?;
        Ok(Self { connection: pool })
    }
    #[tracing::instrument(level = "info")]
    #[cfg(test)]
    pub async fn new() -> Result<Self, Error> {
        let pool = sqlx::sqlite::SqlitePool::connect("sqlite::memory:").await?;
        match sqlx::migrate!().run(&pool).await {
            Ok(_) => {
                dbg!("Migrations ran successfully");
            }
            Err(e) => {
                dbg!(e);
            }
        }
        Ok(Self { connection: pool })
    }

    #[cfg(test)]
    pub async fn close(self) -> Result<(), Error> {
        self.connection.close().await;
        Ok(())
    }
}

impl RobberyDatabase for Database {
    async fn get_last_bought_robbery(&self, user_id: u64) -> Result<DateTime<Utc>, Error> {
        let user = user_id;
        let last_daily = sqlx::query_as::<_, BoughtRobbery>(
            "SELECT last_bought FROM bought_robberies WHERE id = $1",
        )
        .bind(user.to_string())
        .fetch_one(&self.connection)
        .await;

        let res = match last_daily {
            Ok(last_daily) => DateTime::<Utc>::from_timestamp(last_daily.last_bought, 0).unwrap(),
            Err(sqlx::Error::RowNotFound) => {
                let user = user_id.to_string();
                let now = (chrono::Utc::now() - chrono::Duration::days(7)).timestamp();
                dbg!(chrono::Utc::now().timestamp(), now);
                sqlx::query("INSERT INTO bought_robberies (id, last_bought) VALUES ($1, $2)")
                    .bind(user)
                    .bind(now)
                    .execute(&self.connection)
                    .await?;
                DateTime::from_timestamp(now, 0).unwrap()
            }
            Err(e) => return Err(e.into()),
        };
        Ok(res)
    }

    async fn bought_robbery(&self, user_id: u64) -> Result<(), Error> {
        sqlx::query("UPDATE bought_robberies SET last_bought = $1 WHERE id = $2")
            .bind(chrono::Utc::now().timestamp())
            .bind(user_id.to_string())
            .execute(&self.connection)
            .await?;
        Ok(())
    }
}

impl ChannelDatabase for Database {
    async fn get_paid_channels(&self) -> Result<Vec<(u64, i32)>, Error> {
        let data = sqlx::query_as::<_, (i64, i32)>("SELECT id, price FROM paid_channels")
            .fetch_all(&self.connection)
            .await?;
        Ok(data
            .iter()
            .map(|(id, price)| (*id as u64, *price))
            .collect())
    }

    async fn set_channel_price(&self, channel_id: u64, price: i32) -> Result<(), Error> {
        sqlx::query("INSERT INTO paid_channels (id, price) VALUES ($1, $2) ON CONFLICT(id) DO UPDATE SET price = $2")
            .bind(channel_id as i64)
            .bind(price)
            .execute(&self.connection)
            .await?;
        Ok(())
    }

    async fn remove_paid_channel(&self, channel_id: u64) -> Result<(), Error> {
        sqlx::query("DELETE FROM paid_channels WHERE id = $1")
            .bind(channel_id as i64)
            .execute(&self.connection)
            .await?;
        Ok(())
    }
}

impl RoleDatabase for Database {
    async fn get_purchasable_roles(&self) -> Result<Vec<PurchaseableRole>, Error> {
        let data = sqlx::query_as::<_, PurchaseableRoleConfig>(
            "SELECT role_id, price, only_one, required_role_id, increment FROM purchaseable_roles",
        )
        .fetch_all(&self.connection)
        .await?;
        Ok(data
            .iter()
            .map(|x| PurchaseableRole {
                role_id: x.role_id as u64,
                price: x.price,
                only_one: x.only_one,
                increment: x.increment,
                required_role_id: x.required_role_id.map(|x| x as u64),
            })
            .collect())
    }

    async fn set_role_price(
        &self,
        role_id: u64,
        price: i32,
        increment: Option<i32>,
        required_role: Option<u64>,
        only_one: Option<bool>,
    ) -> Result<(), Error> {
        if price == 0 {
            sqlx::query("DELETE FROM purchaseable_roles WHERE role_id = $1")
                .bind(role_id as i64)
                .execute(&self.connection)
                .await?;
            return Ok(());
        }
        let required = required_role.map(|required| required as i64);
        sqlx::query("INSERT INTO purchaseable_roles (role_id, price, increment, required_role_id, only_one) VALUES ($1, $2, $3, $4, $5) ON CONFLICT(role_id) DO UPDATE SET price = $2, increment = $3, required_role_id = $4, only_one = $5")
            .bind(role_id as i64)
            .bind(price)
            .bind(increment)
            .bind(required)
            .bind(only_one)
            .execute(&self.connection)
            .await?;

        Ok(())
    }

    async fn toggle_role_unique(&self, role_id: u64, only_one: bool) -> Result<(), Error> {
        sqlx::query("INSERT INTO purchaseable_roles (role_id, only_one) VALUES ($1, $2) ON CONFLICT(role_id) DO UPDATE SET only_one = $2")
            .bind(role_id as i64)
            .bind(only_one)
            .execute(&self.connection)
            .await?;
        Ok(())
    }

    async fn increment_role_price(&self, role_id: String) -> Result<(), Error> {
        sqlx::query(
            "UPDATE purchaseable_roles SET price = price+COALESCE(increment,0) WHERE role_id = $1",
        )
        .bind(role_id)
        .execute(&self.connection)
        .await?;
        Ok(())
    }

    async fn get_unique_role_holder(&self, role_id: u64) -> Result<Option<RoleHolder>, Error> {
        let a = sqlx::query_as::<_, (i64, DateTime<Utc>)>(
            "SELECT user_id, purchased FROM role_holders WHERE role_id = $1",
        )
        .bind(role_id as i64)
        .fetch_optional(&self.connection)
        .await?;
        match a {
            None => Ok(None),
            Some(rh) => Ok(Some(RoleHolder {
                user_id: rh.0 as u64,
                purchased: rh.1,
            })),
        }
    }

    async fn set_unique_role_holder(&self, role_id: u64, user_id: u64) -> Result<(), Error> {
        sqlx::query("INSERT INTO role_holders (role_id, user_id, purchased) VALUES ($1, $2, CURRENT_TIMESTAMP) ON CONFLICT(role_id) DO UPDATE SET user_id = $2, purchased = CURRENT_TIMESTAMP")
            .bind(role_id as i64)
            .bind(user_id as i64)
            .execute(&self.connection)
            .await?;
        Ok(())
    }

    async fn get_price_decay_config(&self) -> Result<Vec<RolePriceDecayConfig>, Error> {
        let data = sqlx::query_as::<_, RolePriceDecay>(
            "SELECT role_id, amount, interval, last_decay, minimum FROM role_price_decay WHERE amount > 0",
        )
        .fetch_all(&self.connection)
        .await?;

        Ok(data
            .iter()
            .map(|x| RolePriceDecayConfig {
                role_id: x.role_id as u64,
                amount: x.amount,
                interval: x.interval,
                minimum: x.minimum,
                last_decay: x.last_decay,
            })
            .collect())
    }

    async fn decay_role_price(
        &self,
        role_id: u64,
        amount: i32,
        minimum: i32,
    ) -> Result<PurchaseableRole, Error> {
        let data = sqlx::query_as::<_, PurchaseableRoleConfig>(
            "UPDATE purchaseable_roles SET price = MAX(price - $2, $3) WHERE role_id = $1 RETURNING role_id, price, only_one, required_role_id, increment",
        )
        .bind(role_id as i64)
        .bind(amount)
        .bind(minimum)
        .fetch_one(&self.connection)
        .await?;
        Ok(PurchaseableRole {
            role_id: data.role_id as u64,
            price: data.price,
            only_one: data.only_one,
            increment: data.increment,
            required_role_id: data.required_role_id.map(|x| x as u64),
        })
    }

    async fn set_price_decay_config(
        &self,
        role_id: u64,
        amount: i32,
        interval: i32,
        minimum: i32,
    ) -> Result<(), Error> {
        sqlx::query("INSERT INTO role_price_decay (role_id, amount, interval, minimum) VALUES ($1, $2, $3, $4) ON CONFLICT(role_id) DO UPDATE SET amount = $2, interval = $3, minimum = $4")
            .bind(role_id as i64)
            .bind(amount)
            .bind(interval)
            .bind(minimum)
            .execute(&self.connection)
            .await?;
        Ok(())
    }

    async fn price_decayed(&self, role_id: u64) -> Result<(), Error> {
        sqlx::query(
            "Update role_price_decay SET last_decay = CURRENT_TIMESTAMP WHERE role_id = $1",
        )
        .bind(role_id as i64)
        .execute(&self.connection)
        .await?;
        Ok(())
    }
}

impl BalanceDatabase for Database {
    #[tracing::instrument(level = "info")]
    async fn get_balance(&self, user_id: u64) -> Result<i32, Error> {
        let balance: Result<Balance, sqlx::Error> =
            sqlx::query_as("SELECT balance FROM balances WHERE id = $1")
                .bind(user_id as i64)
                .fetch_one(&self.connection)
                .await;

        let result = match balance {
            Ok(user_balance) => user_balance.balance,
            Err(sqlx::Error::RowNotFound) => {
                let _ = sqlx::query("INSERT INTO balances (id, balance) VALUES ($1, $2)")
                    .bind(user_id as i64)
                    .bind(50)
                    .execute(&self.connection)
                    .await?;
                50
            }
            Err(e) => return Err(e.into()),
        };
        Ok(result)
    }

    #[tracing::instrument(level = "debug")]
    async fn award_balances(&self, user_ids: Vec<u64>, award: i32) -> Result<(), Error> {
        if user_ids.is_empty() {
            return Ok(());
        }
        let a = user_ids
            .iter()
            .map(|x| format!("'{}'", x))
            .collect::<Vec<String>>()
            .join(", ");

        sqlx::query(
            format!(
                "UPDATE balances SET balance = balance + $1 WHERE id IN ({})",
                a
            )
            .as_str(),
        )
        .bind(award)
        .execute(&self.connection)
        .await?;
        Ok(())
    }

    #[tracing::instrument(level = "info")]
    async fn subtract_balances(&self, user_ids: Vec<u64>, amount: i32) -> Result<(), Error> {
        if user_ids.is_empty() {
            return Ok(());
        }
        let a = user_ids
            .iter()
            .map(|x| format!("'{}'", x))
            .collect::<Vec<String>>()
            .join(", ");
        sqlx::query(
            format!(
                "UPDATE balances SET balance = balance - $1 WHERE id IN ({})",
                a
            )
            .as_str(),
        )
        .bind(amount)
        .execute(&self.connection)
        .await?;
        Ok(())
    }

    #[tracing::instrument(level = "info")]
    async fn get_leaderboard(&self) -> Result<Vec<(u64, i32)>, Error> {
        let data = sqlx::query_as::<_, (i64, i32)>(
            "SELECT id, balance FROM balances ORDER BY balance DESC LIMIT 10",
        )
        .fetch_all(&self.connection)
        .await?;
        Ok(data
            .iter()
            .map(|(id, balance)| (*id as u64, *balance))
            .collect())
    }

    #[tracing::instrument(level = "info")]
    async fn get_last_daily(&self, user_id: u64) -> Result<Option<DateTime<Utc>>, Error> {
        let data = sqlx::query_as::<_, Daily>("SELECT last_daily FROM dailies WHERE id = $1")
            .bind(user_id as i64)
            .fetch_optional(&self.connection)
            .await?;

        match data {
            Some(last_daily) => Ok(Some(last_daily.last_daily)),
            None => Ok(None),
        }
    }

    #[tracing::instrument(level = "info")]
    async fn did_daily(&self, user_id: u64) -> Result<(), Error> {
        sqlx::query("INSERT INTO DAILIES (id, last_daily) VALUES ($1, $2) ON CONFLICT(id) DO UPDATE SET last_daily = $2")
            .bind(user_id as i64)
            .bind(chrono::Utc::now().timestamp())
            .execute(&self.connection)
            .await?;
        Ok(())
    }

    #[tracing::instrument(level = "info")]
    async fn get_total(&self) -> Result<i32, Error> {
        Ok(
            sqlx::query_as::<_, Total>("SELECT SUM(balance) as total FROM balances")
                .fetch_one(&self.connection)
                .await?
                .total as i32,
        )
    }

    #[tracing::instrument(level = "info")]
    async fn get_avg_balance(&self) -> Result<f32, Error> {
        Ok(sqlx::query_as::<_, Average>(
            "SELECT AVG(balance) as average FROM balances WHERE balance > 0",
        )
        .fetch_one(&self.connection)
        .await?
        .average)
    }

    #[tracing::instrument(level = "info")]
    async fn get_zero_balance(&self) -> Result<i32, Error> {
        Ok(
            sqlx::query_as::<_, Total>("SELECT count(id) as total FROM balances WHERE balance = 0")
                .fetch_one(&self.connection)
                .await?
                .total as i32,
        )
    }

    #[tracing::instrument(level = "info")]
    async fn bury_balance(&self, user_id: u64, amount: i32) -> Result<(), Error> {
        sqlx::query("INSERT INTO buried_balances (id, amount) VALUES ($1, $2) ON CONFLICT(id) DO UPDATE SET amount = amount + $1")
            .bind(user_id.to_string())
            .bind(amount)
            .execute(&self.connection)
            .await?;

        Ok(())
    }

    #[tracing::instrument(level = "info")]
    async fn get_dailies_today(&self) -> Result<i32, Error> {
        let time = chrono::Utc::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        Ok(sqlx::query_as::<_, Total>(
            "SELECT count(id) as total FROM dailies where last_daily > $1",
        )
        .bind(time.and_utc().timestamp())
        .fetch_one(&self.connection)
        .await?
        .total as i32)
    }

    async fn get_crown_leaderboard(&self) -> Result<Vec<(u64, f32)>, Error> {
        let data = sqlx::query_as::<_, (i64, f32)>(
            "SELECT id, hours_held FROM crown_holder_times ORDER BY hours_held DESC LIMIT 10",
        )
        .fetch_all(&self.connection)
        .await?;

        let b = data
            .iter()
            .map(|(id, hours)| (*id as u64, *hours))
            .collect();

        Ok(b)
    }

    async fn update_crown_timer(&self, user_id: u64, hours: f32) -> Result<(), Error> {
        sqlx::query("INSERT INTO crown_holder_times (id, hours_held) VALUES ($1, $2) ON CONFLICT(id) DO UPDATE SET hours_held = hours_held + $2")
            .bind(user_id as i64)
            .bind(hours)
            .execute(&self.connection)
            .await?;

        Ok(())
    }
}
