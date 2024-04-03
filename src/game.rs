use rand::{seq::SliceRandom, Rng};
use std::time;

use crate::commands::HeadsOrTail;

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

#[derive(Debug)]
pub struct CoinGame {
    pub id: String,
    pub players: Vec<String>,
    pub heads: Vec<String>,
    pub tails: Vec<String>,
    pub amount: i32,
    pub pot: i32,
    pub deadline: time::Instant,
}

impl CoinGame {
    pub fn new(
        id: String,
        game_starter: String,
        choice: HeadsOrTail,
        amount: i32,
        deadline: time::Instant,
    ) -> Self {
        let mut heads = vec![];
        let mut tails = vec![];
        let players = vec![game_starter.clone()];

        match choice {
            HeadsOrTail::Heads => heads.push(game_starter),
            HeadsOrTail::Tails => tails.push(game_starter),
        }
        Self {
            id,
            players,
            heads,
            tails,
            amount,
            pot: amount,
            deadline,
        }
    }

    pub fn player_joined(&mut self, player: String, choice: &String) {
        dbg!(&choice);
        self.players.push(player.clone());
        if choice == "Heads" {
            self.heads.push(player);
        } else {
            self.tails.push(player);
        }
        self.pot += self.amount;
    }

    pub fn get_winner(&self) -> String {
        let num = rand::thread_rng().gen_range(0..100);
        if num < 2 {
            return "side".to_owned();
        }
        ["heads", "tails"]
            .choose(&mut rand::thread_rng())
            .unwrap()
            .to_string()
    }
}
