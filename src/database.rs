use std::{
    rc::Rc,
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::{DateTime, Utc};
use rusqlite::{params, types::Value};
use tokio::fs;
use tokio_rusqlite::Connection;

use crate::Error;

pub trait BalanceDatabase {
    async fn get_balance(&self, user_id: String) -> Result<i32, Error>;
    async fn set_balance(&self, user_id: String, balance: i32) -> Result<(), Error>;
    async fn award_balances(&self, user_ids: Vec<String>, award: i32) -> Result<(), Error>;
    async fn subtract_balances(&self, user_ids: Vec<String>, amount: i32) -> Result<(), Error>;
    async fn get_leaderboard(&self) -> Result<Vec<(String, i32)>, Error>;
    async fn get_last_daily(&self, user_id: String) -> Result<DateTime<Utc>, Error>;
    async fn did_daily(&self, user_id: String) -> Result<(), Error>;
    async fn get_total(&self) -> Result<i32, Error>;
    async fn get_avg_balance(&self) -> Result<i32, Error>;
    async fn get_zero_balance(&self) -> Result<i32, Error>;
    async fn get_leader(&self) -> Result<String, Error>;
    async fn bury_balance(&self, user_id: String, amount: i32) -> Result<(), Error>;
    async fn get_dailies_today(&self) -> Result<i32, Error>;
    async fn get_last_bought_robbery(&self, user_id: String) -> Result<DateTime<Utc>, Error>;
    async fn bought_robbery(&self, user_id: String) -> Result<(), Error>;
}

#[derive(Debug)]
pub struct Database {
    connection: tokio_rusqlite::Connection,
}

impl Database {
    #[tracing::instrument(level = "info")]
    pub async fn new() -> Result<Self, Error> {
        fs::create_dir_all("./data").await?;
        let db = Connection::open("./data/johnny.db").await.unwrap();
        db.call(|conn| {
            conn.execute(
                "CREATE TABLE IF NOT EXISTS balances (
            id TEXT PRIMARY KEY,
            balance INTEGER NOT NULL
       )",
                [],
            )?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_balances_balance ON balances (balance)",
                [],
            )?;

            conn.execute(
                "
                CREATE TABLE IF NOT EXISTS dailies (
                    id TEXT PRIMARY KEY,
                    last_daily INTEGER NOT NULL
                )
                ",
                [],
            )?;

            conn.execute(
                "CREATE TABLE IF NOT EXISTS buried_balances (
            id TEXT PRIMARY KEY,
            amount INTEGER NOT NULL
       )",
                [],
            )?;
            conn.execute(
                "
                CREATE TABLE IF NOT EXISTS bought_robberies (
                    id TEXT PRIMARY KEY,
                    last_bought INTEGER NOT NULL
                )
                ",
                [],
            )?;
            rusqlite::vtab::array::load_module(conn)?;
            Ok(())
        })
        .await
        .unwrap();
        Ok(Self { connection: db })
    }
}

impl BalanceDatabase for Database {
    #[tracing::instrument(level = "info")]
    async fn get_balance(&self, user_id: String) -> Result<i32, Error> {
        let user = user_id.clone();
        let balance = self
            .connection
            .call(move |conn| {
                let mut stmt =
                    conn.prepare_cached("SELECT balance FROM balances WHERE id = (?1)")?;
                Ok(stmt.query_row(params![user], |row| {
                    let balance: i32 = row.get(0)?;

                    Ok(balance)
                }))
            })
            .await?;
        let result = match balance {
            Ok(user_balance) => user_balance,
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                let user = user_id;
                let _ = self
                    .connection
                    .call(move |conn| {
                        let mut stmt = conn
                            .prepare_cached("INSERT INTO balances (id, balance) VALUES (?1, ?2)")?;
                        Ok(stmt.query_row(params![user, 50], |row| {
                            let balance: i32 = row.get(0)?;

                            Ok(balance)
                        }))
                    })
                    .await?;
                50
            }
            Err(e) => return Err(e.into()),
        };
        Ok(result)
    }

    #[tracing::instrument(level = "info")]
    async fn bury_balance(&self, user_id: String, amount: i32) -> Result<(), Error> {
        let _ = self
            .connection
            .call(move |conn| {
                let mut stmt = conn.prepare_cached(
                    "INSERT INTO buried_balances (id, amount) VALUES (?1, ?2) ON CONFLICT(id) DO UPDATE SET amount = amount + ?2",
                )?;
                Ok(stmt.execute(params![user_id, amount]))
            })
            .await?;
        Ok(())
    }

    #[tracing::instrument(level = "info")]
    async fn get_leaderboard(&self) -> Result<Vec<(String, i32)>, Error> {
        let leaderboard = self
            .connection
            .call(|conn| {
                let mut stmt = conn.prepare_cached(
                    "SELECT id, balance FROM balances ORDER BY balance DESC LIMIT 10",
                )?;
                let people = stmt
                    .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
                    .collect::<std::result::Result<Vec<(String, i32)>, rusqlite::Error>>();
                Ok(people)
            })
            .await
            .unwrap()?;
        Ok(leaderboard)
    }

    #[tracing::instrument(level = "info")]
    async fn set_balance(&self, user_id: String, balance: i32) -> Result<(), Error> {
        let _ = self
            .connection
            .call(move |conn| {
                let mut stmt =
                    conn.prepare_cached("UPDATE balances SET balance = (?1) WHERE id = (?2)")?;
                Ok(stmt.execute(params![balance, user_id]))
            })
            .await?;
        Ok(())
    }

    #[tracing::instrument(level = "info")]
    async fn award_balances(&self, user_ids: Vec<String>, award: i32) -> Result<(), Error> {
        let _ = self
            .connection
            .call(move |conn| {
                let mut stmt = conn.prepare_cached(
                    "UPDATE balances SET balance = balance + ?1 WHERE id IN rarray(?2)",
                )?;

                let values = Rc::new(
                    user_ids
                        .iter()
                        .map(|a| a.to_string())
                        .map(Value::from)
                        .collect::<Vec<Value>>(),
                );
                stmt.raw_bind_parameter(1, award)?;
                stmt.raw_bind_parameter(2, values)?;
                Ok(stmt.raw_execute().unwrap())
            })
            .await?;
        Ok(())
    }

    #[tracing::instrument(level = "info")]
    async fn subtract_balances(&self, user_ids: Vec<String>, amount: i32) -> Result<(), Error> {
        let _ = self
            .connection
            .call(move |conn| {
                let mut stmt = conn.prepare_cached(
                    "UPDATE balances SET balance = balance - ?1 WHERE id IN rarray(?2)",
                )?;

                let values = Rc::new(
                    user_ids
                        .iter()
                        .map(|a| a.to_string())
                        .map(Value::from)
                        .collect::<Vec<Value>>(),
                );
                stmt.raw_bind_parameter(1, amount)?;
                stmt.raw_bind_parameter(2, values)?;
                Ok(stmt.raw_execute().unwrap())
            })
            .await?;
        Ok(())
    }

    #[tracing::instrument(level = "info")]
    async fn get_last_daily(&self, user_id: String) -> Result<DateTime<Utc>, Error> {
        let user = user_id.clone();
        let last_daily = self
            .connection
            .call(move |conn| {
                let mut stmt =
                    conn.prepare_cached("SELECT last_daily FROM dailies WHERE id = (?1)")?;
                Ok(stmt.query_row(params![user], |row| {
                    let ts: i64 = row.get(0)?;
                    Ok(ts)
                }))
            })
            .await?;

        let res = match last_daily {
            Ok(last_daily) => DateTime::<Utc>::from_timestamp(last_daily, 0).unwrap(),
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                let user = user_id;
                let now = (chrono::Utc::now() - chrono::Duration::days(1)).timestamp();
                dbg!(chrono::Utc::now().timestamp(), now);
                let _ = self
                    .connection
                    .call(move |conn| {
                        let mut stmt = conn.prepare_cached(
                            "INSERT INTO dailies (id, last_daily) VALUES (?1, ?2)",
                        )?;
                        Ok(stmt.execute(params![user, now]))
                    })
                    .await?;
                DateTime::from_timestamp(now, 0).unwrap()
            }
            Err(e) => return Err(e.into()),
        };
        Ok(res)
    }

    #[tracing::instrument(level = "info")]
    async fn did_daily(&self, user_id: String) -> Result<(), Error> {
        let _ = self
            .connection
            .call(move |conn| {
                let user = user_id;
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                let mut stmt =
                    conn.prepare_cached("UPDATE dailies SET last_daily = (?1) WHERE id = (?2)")?;
                Ok(stmt.execute(params![now, user]))
            })
            .await?;
        Ok(())
    }

    #[tracing::instrument(level = "info")]
    async fn get_total(&self) -> Result<i32, Error> {
        Ok(self
            .connection
            .call(move |conn| {
                let mut stmt = conn.prepare_cached("SELECT SUM(balance) FROM balances")?;
                let v = stmt.query_row([], |row| {
                    let total: i32 = row.get(0).unwrap();

                    Ok(total)
                });
                Ok(v.unwrap())
            })
            .await
            .unwrap())
    }

    #[tracing::instrument(level = "info")]
    async fn get_avg_balance(&self) -> Result<i32, Error> {
        Ok(self
            .connection
            .call(move |conn| {
                let mut stmt =
                    conn.prepare_cached("SELECT AVG(balance) FROM balances where balance > 0")?;
                let v = stmt.query_row([], |row| {
                    let total: f32 = row.get(0).unwrap();
                    Ok(total as i32)
                });
                Ok(v.unwrap())
            })
            .await
            .unwrap())
    }

    #[tracing::instrument(level = "info")]
    async fn get_zero_balance(&self) -> Result<i32, Error> {
        Ok(self
            .connection
            .call(move |conn| {
                let mut stmt =
                    conn.prepare_cached("SELECT count(id) FROM balances where balance = 0")?;
                let v = stmt.query_row([], |row| {
                    let total: i32 = row.get(0).unwrap();

                    Ok(total)
                });
                Ok(v.unwrap())
            })
            .await
            .unwrap())
    }

    #[tracing::instrument(level = "info")]
    async fn get_leader(&self) -> Result<String, Error> {
        Ok(self
            .connection
            .call(move |conn| {
                let mut stmt =
                    conn.prepare_cached("SELECT id FROM balances ORDER BY balance DESC LIMIT 1")?;
                let v = stmt.query_row([], |row| {
                    let id: String = row.get(0).unwrap();
                    Ok(id)
                });
                Ok(v.unwrap())
            })
            .await
            .unwrap())
    }

    #[tracing::instrument(level = "info")]
    async fn get_dailies_today(&self) -> Result<i32, Error> {
        Ok(self
            .connection
            .call(move |conn| {
                let mut stmt =
                    conn.prepare_cached("SELECT count(id) FROM dailies where last_daily > ?")?;
                let v = stmt.query_row(
                    [chrono::Utc::now()
                        .date_naive()
                        .and_hms_opt(0, 0, 0)
                        .unwrap()
                        .and_utc()
                        .timestamp()],
                    |row| {
                        let total: i32 = row.get(0).unwrap();
                        Ok(total)
                    },
                );
                Ok(v.unwrap())
            })
            .await
            .unwrap())
    }

    async fn get_last_bought_robbery(&self, user_id: String) -> Result<DateTime<Utc>, Error> {
        let user = user_id.clone();
        let last_daily = self
            .connection
            .call(move |conn| {
                let mut stmt = conn
                    .prepare_cached("SELECT last_bought FROM bought_robberies WHERE id = (?1)")?;
                Ok(stmt.query_row(params![user], |row| {
                    let ts: i64 = row.get(0)?;
                    Ok(ts)
                }))
            })
            .await?;

        let res = match last_daily {
            Ok(last_daily) => DateTime::<Utc>::from_timestamp(last_daily, 0).unwrap(),
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                let user = user_id;
                let now = (chrono::Utc::now() - chrono::Duration::days(7)).timestamp();
                dbg!(chrono::Utc::now().timestamp(), now);
                let _ = self
                    .connection
                    .call(move |conn| {
                        let mut stmt = conn.prepare_cached(
                            "INSERT INTO bought_robberies (id, last_bought) VALUES (?1, ?2)",
                        )?;
                        Ok(stmt.execute(params![user, now]))
                    })
                    .await?;
                DateTime::from_timestamp(now, 0).unwrap()
            }
            Err(e) => return Err(e.into()),
        };
        Ok(res)
    }

    async fn bought_robbery(&self, user_id: String) -> Result<(), Error> {
        let _ = self
            .connection
            .call(move |conn| {
                let user = user_id;
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                let mut stmt = conn.prepare_cached(
                    "UPDATE bought_robberies SET last_bought = (?1) WHERE id = (?2)",
                )?;
                Ok(stmt.execute(params![now, user]))
            })
            .await?;
        Ok(())
    }
}
