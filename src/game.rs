use rand::{seq::SliceRandom, Rng};
use std::time;

use crate::{
    commands::coingamble::HeadsOrTail,
    database::{self, BalanceDatabase, RoleDatabase},
};

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
pub enum GameError {
    PlayerAlreadyJoined,
    PlayerCantAfford,
}

#[derive(Clone, Debug)]
pub enum PossibleResults {
    Heads,
    Tails,
    Side,
}

impl PossibleResults {
    pub fn to_uppercase(&self) -> String {
        match self {
            PossibleResults::Heads => "HEADS".to_owned(),
            PossibleResults::Tails => "TAILS".to_owned(),
            PossibleResults::Side => "SIDE".to_owned(),
        }
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
    pub side_chance: i32,
}

pub struct CoinGameResult {
    pub result: PossibleResults,
    pub winners: Vec<String>,
    pub prize: i32,
    pub prize_with_multiplier: i32,
    pub johnnys_multiplier: Option<f32>,
    pub leader: Option<String>,
    pub remainder: Option<i32>,
}

impl CoinGame {
    pub fn new(
        id: String,
        game_starter: String,
        choice: HeadsOrTail,
        amount: i32,
        deadline: time::Instant,
        side_chance: i32,
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
            side_chance,
        }
    }

    pub async fn player_joined(
        &mut self,
        db: &impl database::BalanceDatabase,
        player: String,
        choice: &String,
    ) -> Result<(), GameError> {
        if self.players.contains(&player) {
            return Err(GameError::PlayerAlreadyJoined);
        }

        let player_balance = db.get_balance(player.clone()).await.unwrap();
        if player_balance < self.amount {
            return Err(GameError::PlayerCantAfford);
        }
        db.subtract_balances(vec![player.clone()], self.amount)
            .await
            .unwrap();
        self.players.push(player.clone());
        if choice == "Heads" {
            self.heads.push(player);
        } else {
            self.tails.push(player);
        }
        self.pot += self.amount;
        Ok(())
    }

    pub async fn get_winner<T: BalanceDatabase + RoleDatabase>(
        &mut self,
        db: &T,
        bot_id: String,
        crown_role_id: i64,
    ) -> CoinGameResult {
        if self.heads.is_empty() {
            self.heads.push(bot_id.clone());
            self.players.push(bot_id.clone());
            self.pot += self.pot;
        } else if self.tails.is_empty() {
            self.tails.push(bot_id.clone());
            self.players.push(bot_id.clone());
            self.pot += self.pot;
        }
        let result = {
            if rand::thread_rng().gen_range(1..=100) <= self.side_chance {
                PossibleResults::Side
            } else {
                let num = rand::thread_rng().gen_range(0..=1);
                if num == 0 {
                    PossibleResults::Heads
                } else {
                    PossibleResults::Tails
                }
            }
        };

        match result {
            PossibleResults::Side => {
                let leaderboard: Vec<String> = db
                    .get_leaderboard()
                    .await
                    .unwrap()
                    .iter()
                    .map(|(u, _b)| u.to_owned())
                    .collect();
                if leaderboard.is_empty() {
                    return CoinGameResult {
                        result,
                        winners: leaderboard,
                        prize: 0,
                        prize_with_multiplier: 0,
                        leader: None,
                        johnnys_multiplier: None,
                        remainder: None,
                    };
                };

                let winner = leaderboard.choose(&mut rand::thread_rng()).unwrap().clone();
                db.award_balances(vec![winner.clone()], self.pot)
                    .await
                    .unwrap();
                CoinGameResult {
                    result,
                    winners: vec![winner],
                    prize: self.pot,
                    prize_with_multiplier: 0,
                    leader: None,
                    johnnys_multiplier: None,
                    remainder: None,
                }
            }
            _ => {
                let winners = match result {
                    PossibleResults::Heads => self.heads.clone(),
                    PossibleResults::Tails => self.tails.clone(),
                    _ => vec![],
                };
                let chance_of_bonus = self.players.len();
                let johnnys_multiplier = if rand::thread_rng().gen_range(0..100) < chance_of_bonus {
                    rand::thread_rng().gen_range(0.20..=2.0)
                } else {
                    0.0
                };

                let prize = self.pot / winners.len() as i32;
                let remainder = self.pot % winners.len() as i32;
                let prize_with_multiplier = prize + (prize as f32 * johnnys_multiplier) as i32;
                let leader =
                    if let Some(user) = db.get_unique_role_holder(crown_role_id).await.unwrap() {
                        db.award_balances(vec![user.user_id.clone()], remainder)
                            .await
                            .unwrap();
                        Some(user.user_id)
                    } else {
                        None
                    };
                if winners[0] != bot_id {
                    db.award_balances(winners.clone(), prize_with_multiplier)
                        .await
                        .unwrap();
                }
                CoinGameResult {
                    result,
                    winners,
                    prize,
                    prize_with_multiplier,
                    leader,
                    johnnys_multiplier: Some(johnnys_multiplier),
                    remainder: Some(remainder),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Error;

    struct TestDb {}
    impl BalanceDatabase for TestDb {
        async fn get_balance(&self, _user_id: String) -> Result<i32, Error> {
            Ok(100)
        }
        async fn subtract_balances(
            &self,
            _user_ids: Vec<String>,
            _amount: i32,
        ) -> Result<(), Error> {
            Ok(())
        }
        async fn award_balances(&self, _user_ids: Vec<String>, _amount: i32) -> Result<(), Error> {
            Ok(())
        }
        async fn get_leaderboard(&self) -> Result<Vec<(String, i32)>, Error> {
            Ok(vec![])
        }

        async fn set_balance(&self, _user_id: String, _balance: i32) -> Result<(), crate::Error> {
            todo!()
        }

        async fn get_last_daily(
            &self,
            _user_id: String,
        ) -> Result<chrono::prelude::DateTime<chrono::prelude::Utc>, crate::Error> {
            todo!()
        }

        async fn did_daily(&self, _user_id: String) -> Result<(), crate::Error> {
            todo!()
        }

        async fn get_total(&self) -> Result<i32, crate::Error> {
            todo!()
        }

        async fn get_avg_balance(&self) -> Result<f32, crate::Error> {
            todo!()
        }

        async fn get_zero_balance(&self) -> Result<i32, crate::Error> {
            todo!()
        }

        async fn get_leader(&self) -> Result<String, crate::Error> {
            todo!()
        }

        async fn bury_balance(&self, _user_id: String, _amount: i32) -> Result<(), crate::Error> {
            todo!()
        }

        async fn get_dailies_today(&self) -> Result<i32, crate::Error> {
            todo!()
        }
    }

    impl RoleDatabase for TestDb {
        async fn get_purchasable_roles(&self) -> Result<Vec<database::PurchaseableRole>, Error> {
            todo!()
        }

        async fn increment_role_price(&self, _role_id: String) -> Result<(), Error> {
            todo!()
        }

        async fn set_role_price(
            &self,
            _role_id: i64,
            _price: i32,
            _increment: Option<i32>,
            _required_role: Option<i64>,
            _only_one: Option<bool>,
        ) -> Result<(), Error> {
            todo!()
        }

        async fn toggle_role_unique(&self, _role_id: i64, _only_one: bool) -> Result<(), Error> {
            todo!()
        }

        async fn get_unique_role_holder(
            &self,
            _role_id: i64,
        ) -> Result<Option<database::UserID>, Error> {
            Ok(None)
        }

        async fn set_unique_role_holder(&self, _role_id: i64, _user_id: &str) -> Result<(), Error> {
            todo!()
        }
    }

    #[tokio::test]
    async fn test_coin_game_get_winner() {
        let mut game = CoinGame {
            id: "1".to_owned(),
            players: vec!["player1".to_owned(), "player2".to_owned()],
            heads: vec!["player1".to_owned()],
            tails: vec!["player2".to_owned()],
            amount: 100,
            pot: 200,
            deadline: time::Instant::now(),
            side_chance: 0,
        };

        let bot_id = "bot".to_owned();
        let crown_role_id = 1;

        let db = TestDb {};
        let mut heads = 0;
        let mut tails = 0;
        let mut side = 0;
        let num_games = 100;

        for _i in 0..num_games {
            match game
                .get_winner(&db, bot_id.clone(), crown_role_id)
                .await
                .result
            {
                PossibleResults::Heads => heads += 1,
                PossibleResults::Tails => tails += 1,
                PossibleResults::Side => side += 1,
            }
        }
        assert_eq!(side, 0);
        assert!(
            (40..=60).contains(&heads),
            "invalid heads amount: {}",
            heads
        );
        assert!(
            (40..=60).contains(&tails),
            "invalid tails amount: {}",
            tails
        );
    }

    #[tokio::test]
    async fn test_coin_game_get_winner_side_percent() {
        let mut game = CoinGame {
            id: "1".to_owned(),
            players: vec!["player1".to_owned(), "player2".to_owned()],
            heads: vec!["player1".to_owned()],
            tails: vec!["player2".to_owned()],
            amount: 100,
            pot: 200,
            deadline: time::Instant::now(),
            side_chance: 10,
        };

        let bot_id = "bot".to_owned();
        let crown_role_id = 1;

        let db = TestDb {};
        let mut side = 0;
        let num_games = 100;

        for _i in 0..num_games {
            match game
                .get_winner(&db, bot_id.clone(), crown_role_id)
                .await
                .result
            {
                PossibleResults::Heads => {}
                PossibleResults::Tails => {}
                PossibleResults::Side => side += 1,
            }
        }

        assert!((5..=15).contains(&side), "invalid side amount: {}", side);
    }

    #[tokio::test]
    async fn test_coin_game_get_winner_adds_bot() {
        let mut game = CoinGame {
            id: "1".to_owned(),
            players: vec!["player1".to_owned(), "player2".to_owned()],
            heads: vec!["player1".to_owned()],
            tails: vec![],
            amount: 100,
            pot: 200,
            deadline: time::Instant::now(),
            side_chance: 0,
        };

        let bot_id = "bot".to_owned();
        let crown_role_id = 1;

        let db = TestDb {};
        let result = game.get_winner(&db, bot_id.clone(), crown_role_id).await;
        assert!(game.tails.contains(&bot_id));
        if let PossibleResults::Tails = result.result {
            assert_eq!(result.winners, vec![bot_id]);
        }
    }
}

#[derive(Debug, Clone)]
pub struct Blackjack {
    pub id: String,
    pub players: Vec<String>,
    pub players_scores: Vec<i32>,
    pub pot: i32,
}

impl Blackjack {
    pub fn new(id: String) -> Self {
        Self {
            id,
            players: vec![],
            players_scores: vec![],
            pot: 0,
        }
    }
    pub fn player_joined(&mut self, player: String) {
        self.players.push(player);
        self.players_scores.push(0);
    }

    pub fn get_winners(&self) -> Vec<String> {
        let mut winners = vec![];
        let mut max_score = 0;
        for (i, score) in self.players_scores.iter().enumerate() {
            if score > &max_score && score <= &21 {
                max_score = *score;
                winners = vec![self.players[i].clone()];
            } else if score == &max_score && score <= &21 {
                winners.push(self.players[i].clone());
            }
        }
        winners
    }

    pub fn get_leaderboard(&self) -> Vec<(String, i32)> {
        self.players
            .iter()
            .zip(self.players_scores.iter())
            .map(|(player, score)| (player.clone(), *score))
            .collect()
    }
}
