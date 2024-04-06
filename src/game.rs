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

    pub fn get_winner(&self, rng: &mut rand::rngs::StdRng) -> String {
        self.players.choose(rng).unwrap().to_string()
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
        self.players.push(player.clone());
        if choice == "Heads" {
            self.heads.push(player);
        } else {
            self.tails.push(player);
        }
        self.pot += self.amount;
    }

    pub fn get_winner(&self, rng: &mut rand::rngs::StdRng) -> String {
        let num = rng.gen_range(0..100);
        if num < 1 {
            "side".to_owned()
        } else if num < 50 {
            "heads".to_owned()
        } else {
            "tails".to_owned()
        }
    }
}

// test
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coin_game_get_winner() {
        let mut rng = rand::SeedableRng::seed_from_u64(42);
        let game = CoinGame {
            id: "1".to_owned(),
            players: vec!["player1".to_owned(), "player2".to_owned()],
            heads: vec!["player1".to_owned()],
            tails: vec!["player2".to_owned()],
            amount: 100,
            pot: 200,
            deadline: time::Instant::now(),
        };
        let winner = game.get_winner(&mut rng);
        assert!(winner == "heads" || winner == "tails" || winner == "side");
    }

    #[test]
    fn test_coin_game_get_winner_odds() {
        let mut rng = rand::SeedableRng::from_entropy();
        let game = CoinGame {
            id: "1".to_owned(),
            players: vec!["player1".to_owned(), "player2".to_owned()],
            heads: vec!["player1".to_owned()],
            tails: vec!["player2".to_owned()],
            amount: 100,
            pot: 200,
            deadline: time::Instant::now(),
        };

        let mut heads = 0;
        let mut tails = 0;
        let mut side = 0;

        let num_games = 1000000;

        for _i in 0..num_games {
            let a = game.get_winner(&mut rng);
            if a == "heads" {
                heads += 1;
            } else if a == "tails" {
                tails += 1;
            } else {
                side += 1;
            }
        }

        assert!(num_games as f64 * 0.03 > side as f64);

        // test that the ratio of heads to tails is close to 1
        assert!((heads as f64 / tails as f64) - 1.0 < 0.01);
    }
}
