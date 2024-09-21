use rand::{seq::SliceRandom, Rng};

use crate::{
    commands::coingamble::HeadsOrTail,
    database::{self, BalanceDatabase, ConfigDatabase, RoleDatabase},
};

#[derive(Debug)]
pub struct Game {
    pub players: Vec<u64>,
    pub amount: i32,
    pub pot: i32,
}

impl Game {
    pub fn new(amount: i32, started_by: u64) -> Self {
        Self {
            players: vec![started_by],
            amount,
            pot: amount,
        }
    }

    pub fn player_joined(&mut self, player: u64) {
        self.players.push(player);
        self.pot += self.amount;
    }

    pub fn get_winner(&self, rng: &mut rand::rngs::StdRng) -> u64 {
        *self.players.choose(rng).unwrap()
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
    pub players: Vec<u64>,
    pub heads: Vec<u64>,
    pub tails: Vec<u64>,
    pub amount: i32,
    pub pot: i32,
    pub side_chance: u32,
    odds_bot_wins: f32,
}

pub struct CoinGameResult {
    pub result: CoinSides,
    pub prize: i32,
    pub prize_with_multiplier: i32,
    pub johnnys_multiplier: Option<f32>,
    pub leader: Option<u64>,
    pub remainder: Option<i32>,
}

impl CoinGame {
    pub fn new(
        game_starter: u64,
        choice: HeadsOrTail,
        amount: i32,
        side_chance: u32,
        bot_odds: f32,
    ) -> Self {
        let mut heads = vec![];
        let mut tails = vec![];
        let players = vec![game_starter];

        match choice {
            HeadsOrTail::Heads => heads.push(game_starter),
            HeadsOrTail::Tails => tails.push(game_starter),
        }
        Self {
            players,
            heads,
            tails,
            amount,
            pot: amount,
            side_chance,
            odds_bot_wins: bot_odds,
        }
    }

    pub async fn player_joined(
        &mut self,
        db: &impl database::BalanceDatabase,
        player: u64,
        choice: &String,
    ) -> Result<(), GameError> {
        if self.players.contains(&player) {
            return Err(GameError::PlayerAlreadyJoined);
        }

        let player_balance = db.get_balance(player).await.unwrap();
        if player_balance < self.amount {
            return Err(GameError::PlayerCantAfford);
        }
        db.subtract_balances(vec![player], self.amount)
            .await
            .unwrap();
        self.players.push(player);
        if choice == "Heads" {
            self.heads.push(player);
        } else {
            self.tails.push(player);
        }
        self.pot += self.amount;
        Ok(())
    }

    pub async fn get_winner<T: BalanceDatabase + RoleDatabase + ConfigDatabase>(
        &mut self,
        db: &T,
        bot_id: u64,
        crown_role_id: u64,
    ) -> CoinGameResult {
        let (heads_odds, tails_odds) = (self.odds_bot_wins, 1.0 - self.odds_bot_wins);
        if self.heads.is_empty() {
            self.heads.push(bot_id);
            self.players.push(bot_id);
            self.pot += self.pot;
        } else if self.tails.is_empty() {
            self.tails.push(bot_id);
            self.players.push(bot_id);
            self.pot += self.pot;
        }
        let result = {
            let mut rng = rand::thread_rng();
            if rng.gen_bool(self.side_chance as f64 / 100.0) {
                CoinSides::Side
            } else {
                [
                    (CoinSides::Heads, heads_odds),
                    (CoinSides::Tails, tails_odds),
                ]
                .choose_weighted(&mut rng, |item| item.1)
                .unwrap()
                .to_owned()
                .0
            }
        };

        match result {
            CoinSides::Side => {
                let current_pot = {
                    db.get_config()
                        .await
                        .unwrap()
                        .lottery_base_prize
                        .unwrap_or(10)
                };

                let new_pot = current_pot + self.pot;

                match db
                    .set_config_value(database::ConfigKey::LotteryBasePrize, &new_pot.to_string())
                    .await
                {
                    Ok(_) => {}
                    Err(e) => tracing::debug!(e),
                };

                CoinGameResult {
                    result,
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
                let leader = if let Ok(Some(user)) = db.get_unique_role_holder(crown_role_id).await
                {
                    match db.award_balances(vec![user.user_id], remainder).await {
                        Ok(_) => {}
                        Err(e) => tracing::debug!(e),
                    };
                    Some(user.user_id)
                } else {
                    None
                };
                if winners[0] != bot_id {
                    match db
                        .award_balances(winners.to_vec(), prize_with_multiplier)
                        .await
                    {
                        Ok(_) => {}
                        Err(e) => tracing::debug!(e),
                    }
                }
                CoinGameResult {
                    result,
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
            players: vec![8222483375454858662, 5607624227456207587],
            heads: vec![8222483375454858662],
            tails: vec![5607624227456207587],
            amount: 100,
            pot: 200,
            side_chance: 0,
            odds_bot_wins: 0.5,
        };

        let bot_id = new_user_id();
        let crown_role_id = 1;

        let db = database::Database::new().await.unwrap();
        let mut heads = 0;
        let mut tails = 0;
        let mut side = 0;
        let num_games = 100;

        for _i in 0..num_games {
            match game.get_winner(&db, bot_id, crown_role_id).await.result {
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
            players: vec![p1, p2],
            heads: vec![p1],
            tails: vec![p2],
            amount: 100,
            pot: 200,
            side_chance: 10,
            odds_bot_wins: 1.0,
        };

        let bot_id = new_user_id();
        let crown_role_id = 1;

        let db = database::Database::new().await.unwrap();
        let mut side = 0;
        let num_games = 100;

        for _i in 0..num_games {
            match game.get_winner(&db, bot_id, crown_role_id).await.result {
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
            players: vec![p1],
            heads: vec![p1],
            tails: vec![],
            amount: 100,
            pot: 100,
            side_chance: 0,
            odds_bot_wins: 1.0,
        };

        let bot_id = new_user_id();
        let crown_role_id = 1;

        let db = database::Database::new().await.unwrap();
        let _ = game.get_winner(&db, bot_id, crown_role_id).await;
        assert!(game.tails.contains(&bot_id));
        assert_eq!(game.pot, 200);
    }

    #[tokio::test]
    async fn test_coin_game_get_winners_award() {
        let (p1, p2) = (new_user_id(), new_user_id());
        let mut game = CoinGame {
            players: vec![p1, p2],
            heads: vec![p1],
            tails: vec![p2],
            amount: 100,
            pot: 200,
            side_chance: 0,
            odds_bot_wins: 1.0,
        };

        let bot_id = new_user_id();
        let crown_role_id = 1;

        let db = database::Database::new().await.unwrap();
        for p in &game.players {
            // sets balance to 50
            db.get_balance(*p).await.unwrap();
        }

        let result = game.get_winner(&db, bot_id, crown_role_id).await;
        if let CoinSides::Tails = result.result {
            let p2_balance = db.get_balance(p2).await.unwrap();
            assert_eq!(p2_balance, 250);
        }
        if let CoinSides::Heads = result.result {
            let p1_balance = db.get_balance(p1).await.unwrap();
            assert_eq!(p1_balance, 250);
        }
        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_coin_game_get_winners_side_award() {
        let (p1, p2) = (new_user_id(), new_user_id());
        let mut game = CoinGame {
            players: vec![p1, p2],
            heads: vec![p1],
            tails: vec![p2],
            amount: 100,
            pot: 200,
            side_chance: 100,
            odds_bot_wins: 1.0,
        };

        let bot_id = new_user_id();
        let crown_role_id = 1;

        let db = database::Database::new().await.unwrap();
        for p in &game.players {
            // sets balance to 50
            db.get_balance(*p).await.unwrap();
        }

        let result = game.get_winner(&db, bot_id, crown_role_id).await;
        assert_eq!(result.result, CoinSides::Side);
        for p in &game.players {
            let balance = db.get_balance(*p).await.unwrap();
            assert_eq!(balance, 50);
        }
        db.close().await.unwrap();
    }

    fn new_user_id() -> u64 {
        rand::thread_rng().gen_range::<u64, _>(0..1000000000000000000)
    }

    #[tokio::test]
    async fn test_coin_game_get_winners_remainder() {
        let p1 = new_user_id();
        let p2 = new_user_id();
        let p3 = new_user_id();
        let p4 = new_user_id();
        let mut game = CoinGame {
            players: vec![p1, p2, p3, p4],
            heads: vec![p1, p2],
            tails: vec![p3, p4],
            amount: 1,
            pot: 11,
            side_chance: 0,
            odds_bot_wins: 1.0,
        };

        let bot_id = new_user_id();
        let crown_role_id = 1;

        let db = database::Database::new().await.unwrap();
        for p in &game.players {
            // sets balance to 50
            db.get_balance(*p).await.unwrap();
        }
        let p5 = new_user_id();
        db.get_balance(p5).await.unwrap();
        db.set_unique_role_holder(crown_role_id, p5).await.unwrap();

        let _ = game.get_winner(&db, bot_id, crown_role_id).await;
        let crown_balance = db.get_balance(p5).await.unwrap();
        assert_eq!(crown_balance, 51);
        db.close().await.unwrap();
    }

    mod lottery {
        use super::super::Lottery;

        #[tokio::test]
        async fn test_lottery_get_winner() {
            let lottery = Lottery::new(vec![(1, 1), (2, 1), (3, 10)]);

            let mut winners = vec![];

            for _i in 0..100 {
                let winner = lottery.get_winner();
                winners.push(winner);
            }

            let num_1 = winners.iter().filter(|&&x| x == 1).count();
            let num_2 = winners.iter().filter(|&&x| x == 2).count();
            let num_3 = winners.iter().filter(|&&x| x == 3).count();

            assert!(num_3 > num_2);
            assert!(num_3 > num_1);
        }

        #[tokio::test]
        async fn test_lottery_get_winner_skewed() {
            let lottery = Lottery::new(vec![(1, 0), (2, 0), (3, 10)]);

            let mut winners = vec![];

            for _i in 0..100 {
                let winner = lottery.get_winner();
                winners.push(winner);
            }

            let num_3 = winners.iter().filter(|&&x| x == 3).count();

            assert_eq!(num_3, 100)
        }
    }
}

#[derive(Debug, Clone)]
pub struct Blackjack {
    pub players: Vec<u64>,
    pub players_scores: Vec<i32>,
    pub pot: i32,
}

impl Blackjack {
    pub fn new() -> Self {
        Self {
            players: vec![],
            players_scores: vec![],
            pot: 0,
        }
    }
    pub fn player_joined(&mut self, player: u64) {
        self.players.push(player);
        self.players_scores.push(0);
    }

    pub fn get_winners(&self) -> Vec<u64> {
        let mut winners = vec![];
        let mut max_score = 0;
        for (i, score) in self.players_scores.iter().enumerate() {
            if score > &max_score && score <= &21 {
                max_score = *score;
                winners = vec![self.players[i]];
            } else if score == &max_score && score <= &21 {
                winners.push(self.players[i]);
            }
        }
        winners
    }

    pub fn get_leaderboard(&self) -> Vec<(u64, i32)> {
        self.players
            .iter()
            .zip(self.players_scores.iter())
            .map(|(player, score)| (*player, *score))
            .collect()
    }
}

pub struct Lottery {
    pub players: Vec<(u64, i32)>,
}

impl Lottery {
    pub fn new(players: Vec<(u64, i32)>) -> Self {
        Self { players }
    }

    pub fn get_winner(&self) -> u64 {
        if self.players.is_empty() {
            return 0;
        }
        self.players
            .choose_weighted(&mut rand::thread_rng(), |item| item.1)
            .unwrap()
            .to_owned()
            .0
    }
}
