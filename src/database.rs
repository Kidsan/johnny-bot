use std::rc::Rc;

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
}

pub struct Database {
    connection: tokio_rusqlite::Connection,
}

impl Database {
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
            rusqlite::vtab::array::load_module(conn)?;
            Ok(())
        })
        .await
        .unwrap();
        Ok(Self { connection: db })
    }
}

impl BalanceDatabase for Database {
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
}
