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

#[derive(Clone, Debug, PartialEq)]
pub enum CoinSides {
    Heads,
    Tails,
    Side,
}

impl CoinSides {
    pub fn to_uppercase(&self) -> String {
        match self {
            CoinSides::Heads => "HEADS".to_owned(),
            CoinSides::Tails => "TAILS".to_owned(),
            CoinSides::Side => "SIDE".to_owned(),
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
    pub result: CoinSides,
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

        let player_balance = db.get_balance(player.parse().unwrap()).await.unwrap();
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
            let mut rng = rand::thread_rng();
            if rng.gen_ratio(self.side_chance.try_into().unwrap(), 100) {
                CoinSides::Side
            } else {
                [CoinSides::Heads, CoinSides::Tails]
                    .choose(&mut rng)
                    .unwrap()
                    .to_owned()
            }
        };

        match result {
            CoinSides::Side => {
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
                    CoinSides::Heads => self.heads.clone(),
                    CoinSides::Tails => self.tails.clone(),
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

    #[tokio::test]
    async fn test_coin_game_get_winner() {
        let mut game = CoinGame {
            id: "1".to_owned(),
            players: vec![
                "8222483375454858662".to_owned(),
                "5607624227456207587".to_owned(),
            ],
            heads: vec!["8222483375454858662".to_owned()],
            tails: vec!["5607624227456207587".to_owned()],
            amount: 100,
            pot: 200,
            deadline: time::Instant::now(),
            side_chance: 0,
        };

        let bot_id = "bot".to_owned();
        let crown_role_id = 1;

        let db = database::Database::new().await.unwrap();
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
                CoinSides::Heads => heads += 1,
                CoinSides::Tails => tails += 1,
                CoinSides::Side => side += 1,
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
        let p1 = new_user_id();
        let p2 = new_user_id();
        let mut game = CoinGame {
            id: "1".to_owned(),
            players: vec![p1.clone(), p2.clone()],
            heads: vec![p1.clone()],
            tails: vec![p2.clone()],
            amount: 100,
            pot: 200,
            deadline: time::Instant::now(),
            side_chance: 10,
        };

        let bot_id = "bot".to_owned();
        let crown_role_id = 1;

        let db = database::Database::new().await.unwrap();
        let mut side = 0;
        let num_games = 100;

        for _i in 0..num_games {
            match game
                .get_winner(&db, bot_id.clone(), crown_role_id)
                .await
                .result
            {
                CoinSides::Heads => {}
                CoinSides::Tails => {}
                CoinSides::Side => side += 1,
            }
        }

        assert!((5..=15).contains(&side), "invalid side amount: {}", side);
    }

    #[tokio::test]
    async fn test_coin_game_get_winner_adds_bot() {
        let p1 = new_user_id();
        let mut game = CoinGame {
            id: "1".to_owned(),
            players: vec![p1.clone()],
            heads: vec![p1],
            tails: vec![],
            amount: 100,
            pot: 100,
            deadline: time::Instant::now(),
            side_chance: 0,
        };

        let bot_id = "bot".to_owned();
        let crown_role_id = 1;

        let db = database::Database::new().await.unwrap();
        let result = game.get_winner(&db, bot_id.clone(), crown_role_id).await;
        assert!(game.tails.contains(&bot_id));
        assert_eq!(game.pot, 200);
        if let CoinSides::Tails = result.result {
            assert_eq!(result.winners, vec![bot_id]);
        }
    }

    #[tokio::test]
    async fn test_coin_game_get_winners_award() {
        let (p1, p2) = (new_user_id(), new_user_id());
        let mut game = CoinGame {
            id: "1".to_owned(),
            players: vec![p1.clone(), p2.clone()],
            heads: vec![p1.clone()],
            tails: vec![p2.clone()],
            amount: 100,
            pot: 200,
            deadline: time::Instant::now(),
            side_chance: 0,
        };

        let bot_id = "bot".to_owned();
        let crown_role_id = 1;

        let db = database::Database::new().await.unwrap();
        for p in &game.players {
            // sets balance to 50
            db.get_balance(p.parse().unwrap()).await.unwrap();
        }

        let result = game.get_winner(&db, bot_id.clone(), crown_role_id).await;
        if let CoinSides::Tails = result.result {
            assert_eq!(result.winners, game.tails);
            let p2_balance = db.get_balance(p2.parse().unwrap()).await.unwrap();
            assert_eq!(p2_balance, 250);
        }
        if let CoinSides::Heads = result.result {
            assert_eq!(result.winners, game.heads);
            let p1_balance = db.get_balance(p1.parse().unwrap()).await.unwrap();
            assert_eq!(p1_balance, 250);
        }
        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_coin_game_get_winners_side_award() {
        let (p1, p2) = (new_user_id(), new_user_id());
        let mut game = CoinGame {
            id: "1".to_owned(),
            players: vec![p1.clone(), p2.clone()],
            heads: vec![p1.clone()],
            tails: vec![p2.clone()],
            amount: 100,
            pot: 200,
            deadline: time::Instant::now(),
            side_chance: 100,
        };

        let bot_id = "bot".to_owned();
        let crown_role_id = 1;

        let db = database::Database::new().await.unwrap();
        for p in &game.players {
            // sets balance to 50
            db.get_balance(p.parse().unwrap()).await.unwrap();
        }

        let result = game.get_winner(&db, bot_id.clone(), crown_role_id).await;
        assert_eq!(result.result, CoinSides::Side);
        assert_eq!(result.winners.len(), 1);
        let winner = &result.winners[0];
        let balance = db.get_balance(winner.parse().unwrap()).await.unwrap();
        assert_eq!(balance, 250);
        if winner.eq_ignore_ascii_case(&p1) {
            let balance = db.get_balance(p2.parse().unwrap()).await.unwrap();
            assert_eq!(balance, 50);
        } else {
            let balance = db.get_balance(p1.parse().unwrap()).await.unwrap();
            assert_eq!(balance, 50);
        }
        db.close().await.unwrap();
    }

    fn new_user_id() -> String {
        rand::thread_rng()
            .gen_range::<i64, _>(0..1000000000000000000)
            .to_string()
    }

    #[tokio::test]
    async fn test_coin_game_get_winners_remainder() {
        let p1 = new_user_id();
        let p2 = new_user_id();
        let p3 = new_user_id();
        let p4 = new_user_id();
        let mut game = CoinGame {
            id: "1".to_owned(),
            players: vec![p1.clone(), p2.clone(), p3.clone(), p4.clone()],
            heads: vec![p1, p2],
            tails: vec![p3, p4],
            amount: 1,
            pot: 11,
            deadline: time::Instant::now(),
            side_chance: 0,
        };

        let bot_id = "bot".to_owned();
        let crown_role_id = 1;

        let db = database::Database::new().await.unwrap();
        for p in &game.players {
            // sets balance to 50
            db.get_balance(p.parse().unwrap()).await.unwrap();
        }
        let p5 = new_user_id();
        db.get_balance(p5.parse().unwrap()).await.unwrap();
        db.set_unique_role_holder(crown_role_id, &p5.clone())
            .await
            .unwrap();

        let result = game.get_winner(&db, bot_id.clone(), crown_role_id).await;
        assert_eq!(result.winners.len(), 2);
        for winner in &result.winners {
            let balance = db.get_balance(winner.parse().unwrap()).await.unwrap();
            assert_eq!(balance, 55);
        }
        let crown_balance = db.get_balance(p5.parse().unwrap()).await.unwrap();
        assert_eq!(crown_balance, 51);
        db.close().await.unwrap();
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
