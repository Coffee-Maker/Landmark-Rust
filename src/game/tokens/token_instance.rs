use std::collections::HashMap;
use std::fs;
use std::ops::Not;

use color_eyre::eyre::{ContextCompat, eyre};
use color_eyre::Result;
use serde::Deserialize;
use toml::Table;
use walkdir::WalkDir;
use crate::game::board::Board;

use crate::game::tokens::token_deserializer::{TokenData, TokenBehavior};
use crate::game::game_communicator::GameCommunicator;
use crate::game::id_types::{TokenInstanceId, LocationId, PlayerId, ServerInstanceId};
use crate::game::state_resources::StateResources;
use crate::game::token_slot::TokenSlot;

#[derive(Clone, Debug)]
pub struct TokenInstance {
    pub token_data: &'static TokenData,
    pub owner: PlayerId,
    pub location: LocationId,
    pub instance_id: TokenInstanceId,
    pub behaviors: Vec<TokenBehavior>,
    pub cost: u32,
    pub base_stats: UnitStats,
    pub current_stats: UnitStats,
    pub equipped_items: Vec<TokenSlot>,
    pub token_types: Vec<String>,
    pub hidden: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct UnitStats {
    pub health: i32,
    pub defense: i32,
    pub attack: i32,
}

impl UnitStats {
    pub fn process_damage(&mut self, amount: i32) {
        self.defense -= amount;
        if self.defense <= 0 {
            self.health += self.defense;
            if self.health <= 0 {
                self.health = 0;
            }
            self.defense = 0;
        }
    }
}

impl TokenInstance {
    pub fn is_alive(&self, resources: &StateResources, board: &Board) -> bool {
        let graveyard_1 = resources.locations.get(&board.side_1.graveyard).unwrap();
        let graveyard_2 = resources.locations.get(&board.side_2.graveyard).unwrap();
        return graveyard_1.contains(self.instance_id) == false && graveyard_2.contains(self.instance_id) == false;
    }
}
