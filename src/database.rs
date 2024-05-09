use chrono::{DateTime, Utc};
use sqlx::Pool;
use tokio::fs;

use crate::Error;

#[derive(Debug, sqlx::FromRow)]
struct Balance {
    id: String,
    balance: i32,
}
#[derive(Debug, sqlx::FromRow)]
struct Daily {
    last_daily: i64,
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
pub struct PurchaseableRole {
    pub role_id: String,
    pub price: i32,
    pub only_one: bool,
    pub increment: Option<i32>,
    pub required_role_id: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct UserID {
    pub user_id: String,
}

pub trait BalanceDatabase {
    async fn get_balance(&self, user_id: String) -> Result<i32, Error>;
    async fn set_balance(&self, user_id: String, balance: i32) -> Result<(), Error>;
    async fn award_balances(&self, user_ids: Vec<String>, award: i32) -> Result<(), Error>;
    async fn subtract_balances(&self, user_ids: Vec<String>, amount: i32) -> Result<(), Error>;
    async fn get_leaderboard(&self) -> Result<Vec<(String, i32)>, Error>;
    async fn get_last_daily(&self, user_id: String) -> Result<DateTime<Utc>, Error>;
    async fn did_daily(&self, user_id: String) -> Result<(), Error>;
    async fn get_total(&self) -> Result<i32, Error>;
    async fn get_avg_balance(&self) -> Result<f32, Error>;
    async fn get_zero_balance(&self) -> Result<i32, Error>;
    async fn get_leader(&self) -> Result<String, Error>;
    async fn bury_balance(&self, user_id: String, amount: i32) -> Result<(), Error>;
    async fn get_dailies_today(&self) -> Result<i32, Error>;
    async fn get_last_bought_robbery(&self, user_id: String) -> Result<DateTime<Utc>, Error>;
    async fn bought_robbery(&self, user_id: String) -> Result<(), Error>;
    async fn get_paid_channels(&self) -> Result<Vec<(i64, i32)>, Error>;
    async fn set_channel_price(&self, channel_id: i64, price: i32) -> Result<(), Error>;
    async fn remove_paid_channel(&self, channel_id: i64) -> Result<(), Error>;
    async fn get_purchasable_roles(&self) -> Result<Vec<PurchaseableRole>, Error>;
    async fn increment_role_price(&self, role_id: String) -> Result<(), Error>;
    async fn set_role_price(
        &self,
        role_id: i64,
        price: i32,
        increment: Option<i32>,
        required_role: Option<i64>,
        only_one: Option<bool>,
    ) -> Result<(), Error>;
    async fn toggle_role_unique(&self, role_id: i64, only_one: bool) -> Result<(), Error>;
    async fn get_unique_role_holder(&self, role_id: i64) -> Result<Option<UserID>, Error>;
    async fn set_unique_role_holder(&self, role_id: i64, user_id: &str) -> Result<(), Error>;
}

#[derive(Debug)]
pub struct Database {
    connection: Pool<sqlx::Sqlite>,
}

impl Database {
    #[tracing::instrument(level = "info")]
    pub async fn new() -> Result<Self, Error> {
        fs::create_dir_all("./data").await?;
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename("./data/johnny.db")
            .optimize_on_close(true, None)
            .create_if_missing(true);

        let pool = sqlx::sqlite::SqlitePool::connect_with(options).await?;
        sqlx::migrate!().run(&pool).await?;
        Ok(Self { connection: pool })
    }
}

impl BalanceDatabase for Database {
    #[tracing::instrument(level = "info")]
    async fn get_balance(&self, user_id: String) -> Result<i32, Error> {
        let user = user_id.clone();
        let balance: Result<Balance, sqlx::Error> =
            sqlx::query_as("SELECT id, balance FROM balances WHERE id = $1")
                .bind(user)
                .fetch_one(&self.connection)
                .await;

        let result = match balance {
            Ok(user_balance) => user_balance.balance,
            Err(sqlx::Error::RowNotFound) => {
                let user = user_id;
                let _ = sqlx::query("INSERT INTO balances (id, balance) VALUES ($1, $2)")
                    .bind(user)
                    .bind(50)
                    .execute(&self.connection)
                    .await?;
                50
            }
            Err(e) => return Err(e.into()),
        };
        Ok(result)
    }

    #[tracing::instrument(level = "info")]
    async fn set_balance(&self, user_id: String, balance: i32) -> Result<(), Error> {
        sqlx::query("UPDATE balances SET balance = $1 WHERE id = $2")
            .bind(balance)
            .bind(user_id)
            .execute(&self.connection)
            .await?;
        Ok(())
    }

    #[tracing::instrument(level = "debug")]
    async fn award_balances(&self, user_ids: Vec<String>, award: i32) -> Result<(), Error> {
        if user_ids.is_empty() {
            return Ok(());
        }
        let a = user_ids.join(", ");

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
    async fn subtract_balances(&self, user_ids: Vec<String>, amount: i32) -> Result<(), Error> {
        if user_ids.is_empty() {
            return Ok(());
        }
        let a = user_ids.join(", ");
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
    async fn get_leaderboard(&self) -> Result<Vec<(String, i32)>, Error> {
        Ok(
            sqlx::query_as("SELECT id, balance FROM balances ORDER BY balance DESC LIMIT 10")
                .fetch_all(&self.connection)
                .await?,
        )
    }

    #[tracing::instrument(level = "info")]
    async fn get_last_daily(&self, user_id: String) -> Result<DateTime<Utc>, Error> {
        let user = user_id.clone();
        let last_daily: Result<Daily, sqlx::Error> =
            sqlx::query_as("SELECT id, last_daily FROM dailies WHERE id = $1")
                .bind(user)
                .fetch_one(&self.connection)
                .await;

        let res = match last_daily {
            Ok(last_daily) => DateTime::<Utc>::from_timestamp(last_daily.last_daily, 0).unwrap(),
            Err(sqlx::Error::RowNotFound) => {
                let user = user_id;
                let now = (chrono::Utc::now() - chrono::Duration::days(1)).timestamp();
                dbg!(chrono::Utc::now().timestamp(), now);
                sqlx::query("INSERT INTO dailies (id, last_daily) VALUES ($1, $2)")
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

    #[tracing::instrument(level = "info")]
    async fn did_daily(&self, user_id: String) -> Result<(), Error> {
        sqlx::query("UPDATE dailies SET last_daily = $1 WHERE id = $2")
            .bind(chrono::Utc::now().timestamp())
            .bind(user_id)
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
    async fn get_leader(&self) -> Result<String, Error> {
        Ok(sqlx::query_as::<_, Balance>(
            "SELECT id, balance FROM balances ORDER BY balance DESC LIMIT 1",
        )
        .fetch_one(&self.connection)
        .await?
        .id)
    }

    #[tracing::instrument(level = "info")]
    async fn bury_balance(&self, user_id: String, amount: i32) -> Result<(), Error> {
        sqlx::query("INSERT INTO buried_balances (id, amount) VALUES ($1, $2) ON CONFLICT(id) DO UPDATE SET amount = amount + $1")
            .bind(user_id)
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

    async fn get_last_bought_robbery(&self, user_id: String) -> Result<DateTime<Utc>, Error> {
        let user = user_id.clone();
        let last_daily = sqlx::query_as::<_, BoughtRobbery>(
            "SELECT last_bought FROM bought_robberies WHERE id = $1",
        )
        .bind(user)
        .fetch_one(&self.connection)
        .await;

        let res = match last_daily {
            Ok(last_daily) => DateTime::<Utc>::from_timestamp(last_daily.last_bought, 0).unwrap(),
            Err(sqlx::Error::RowNotFound) => {
                let user = user_id;
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

    async fn bought_robbery(&self, user_id: String) -> Result<(), Error> {
        sqlx::query("UPDATE bought_robberies SET last_bought = 1 WHERE id = $2")
            .bind(chrono::Utc::now().timestamp())
            .bind(user_id)
            .execute(&self.connection)
            .await?;
        Ok(())
    }

    async fn get_paid_channels(&self) -> Result<Vec<(i64, i32)>, Error> {
        Ok(sqlx::query_as("SELECT id, price FROM paid_channels")
            .fetch_all(&self.connection)
            .await?)
    }

    async fn set_channel_price(&self, channel_id: i64, price: i32) -> Result<(), Error> {
        sqlx::query("INSERT INTO paid_channels (id, price) VALUES ($1, $2) ON CONFLICT(id) DO UPDATE SET price = $2")
            .bind(channel_id)
            .bind(price)
            .execute(&self.connection)
            .await?;
        Ok(())
    }

    async fn remove_paid_channel(&self, channel_id: i64) -> Result<(), Error> {
        sqlx::query("DELETE FROM paid_channels WHERE id = $1")
            .bind(channel_id)
            .execute(&self.connection)
            .await?;
        Ok(())
    }

    async fn get_purchasable_roles(&self) -> Result<Vec<PurchaseableRole>, Error> {
        Ok(sqlx::query_as(
            "SELECT role_id, price, only_one, required_role_id, increment FROM purchaseable_roles",
        )
        .fetch_all(&self.connection)
        .await?)
    }

    async fn set_role_price(
        &self,
        role_id: i64,
        price: i32,
        increment: Option<i32>,
        required_role: Option<i64>,
        only_one: Option<bool>,
    ) -> Result<(), Error> {
        if price == 0 {
            sqlx::query("DELETE FROM purchaseable_roles WHERE role_id = $1")
                .bind(role_id)
                .execute(&self.connection)
                .await?;
            return Ok(());
        }
        sqlx::query("INSERT INTO purchaseable_roles (role_id, price, increment, required_role_id, only_one) VALUES ($1, $2, $3, $4, $5) ON CONFLICT(role_id) DO UPDATE SET price = $2, increment = $3, required_role_id = $4, only_one = $5")
            .bind(role_id)
            .bind(price)
            .bind(increment)
            .bind(required_role)
            .bind(only_one)
            .execute(&self.connection)
            .await?;

        Ok(())
    }

    async fn toggle_role_unique(&self, role_id: i64, only_one: bool) -> Result<(), Error> {
        sqlx::query("INSERT INTO purchaseable_roles (role_id, only_one) VALUES ($1, $2) ON CONFLICT(role_id) DO UPDATE SET only_one = $2")
            .bind(role_id)
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

    async fn get_unique_role_holder(&self, role_id: i64) -> Result<Option<UserID>, Error> {
        Ok(
            sqlx::query_as::<_, UserID>("SELECT user_id FROM role_holders WHERE role_id = $1")
                .bind(role_id)
                .fetch_optional(&self.connection)
                .await?,
        )
    }

    async fn set_unique_role_holder(&self, role_id: i64, user_id: &str) -> Result<(), Error> {
        sqlx::query("INSERT INTO role_holders (role_id, user_id) VALUES ($1, $2) ON CONFLICT(role_id) DO UPDATE SET user_id = $2")
            .bind(role_id)
            .bind(user_id)
            .execute(&self.connection)
            .await?;
        Ok(())
    }
}
