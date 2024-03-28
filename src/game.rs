use rand::seq::SliceRandom;
use std::time;

#[derive(Debug)]
pub struct Game {
    pub id: String,
    pub players: Vec<String>,
    pub amount: i32,
    pub pot: i32,
    pub deadline: time::Instant,
}

impl Game {
    pub fn new(id: String, amount: i32, started_by: String, deadline: time::Instant) -> Self {
        Self {
            id,
            players: vec![started_by],
            amount,
            pot: amount,
            deadline,
        }
    }

    pub fn player_joined(&mut self, player: String) {
        self.players.push(player);
        self.pot += self.amount;
    }

    pub fn get_winner(&self) -> String {
        self.players
            .choose(&mut rand::thread_rng())
            .unwrap()
            .to_string()
    }
}
