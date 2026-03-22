pub mod texas_holdem_v1;

use std::collections::HashMap;

use crate::domain::game::Game;

pub struct GameManifest {
    pub game_type: String,
    pub display_name: String,
    pub min_players: usize,
    pub max_players: usize,
    pub turn_timeout_ms: u64,
}

pub struct GameRegistry {
    games: HashMap<String, Box<dyn Game>>,
}

impl GameRegistry {
    pub fn new() -> Self {
        Self {
            games: HashMap::new(),
        }
    }

    pub fn register(&mut self, game: Box<dyn Game>) {
        self.games.insert(game.game_type().to_string(), game);
    }

    pub fn get(&self, game_type: &str) -> Option<&dyn Game> {
        self.games.get(game_type).map(|g| g.as_ref())
    }

    pub fn list(&self) -> Vec<GameManifest> {
        self.games
            .values()
            .map(|g| GameManifest {
                game_type: g.game_type().to_string(),
                display_name: g.display_name().to_string(),
                min_players: g.min_players(),
                max_players: g.max_players(),
                turn_timeout_ms: g.turn_timeout_ms(),
            })
            .collect()
    }
}

impl Default for GameRegistry {
    fn default() -> Self {
        Self::new()
    }
}
