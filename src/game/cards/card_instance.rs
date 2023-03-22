use std::collections::HashMap;
use std::fs;
use std::ops::Not;

use color_eyre::eyre::{ContextCompat, eyre};
use color_eyre::Result;
use serde::Deserialize;
use toml::Table;
use walkdir::WalkDir;
use crate::game::board::Board;

use crate::game::cards::card_deserialization::{Card, CardBehavior};
use crate::game::id_types::{CardInstanceId, LocationId, PlayerId, ServerInstanceId};
use crate::game::state_resources::StateResources;

#[derive(Clone)]
pub struct CardInstance {
    pub card: &'static Card,
    pub owner: PlayerId,
    pub location: LocationId,
    pub instance_id: CardInstanceId,
    pub behaviors: Vec<CardBehavior>,
    pub cost: i32,
    pub stats: UnitStats,
    pub card_types: Vec<String>,
}

#[derive(Clone, Copy)]
pub struct UnitStats {
    pub health: i32,
    pub defense: i32,
    pub attack: i32,
}

impl CardInstance {
    pub fn is_alive(&self, resources: &StateResources, board: &Board) -> bool {
        let graveyard_1 = resources.locations.get(&board.side_1.graveyard).unwrap();
        let graveyard_2 = resources.locations.get(&board.side_2.graveyard).unwrap();
        return graveyard_1.contains(self.instance_id) == false && graveyard_2.contains(self.instance_id) == false;
    }
}
