use chrono::TimeDelta;

use crate::{database, RoleDatabase};

#[derive(Debug)]
pub struct Johnny {
    db: database::Database,
    pub t: std::sync::mpsc::Sender<(i64, i32)>,
}

impl Johnny {
    pub fn new(db: database::Database, t: std::sync::mpsc::Sender<(i64, i32)>) -> Self {
        Self { db, t }
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

            if time_passed % 300 == 0 {
                self.decay().await;
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
                dbg!("Decaying price", &config.role_id, now, last, config.amount);
                match self
                    .db
                    .decay_role_price(config.role_id, config.amount, config.minimum)
                    .await
                {
                    Ok(r) => {
                        self.t.send((r.role_id.parse().unwrap(), r.price)).unwrap();
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
}
