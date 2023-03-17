use color_eyre::eyre::{eyre};
use std::collections::HashMap;
use std::fs;

use crate::game::game_state::{CardKey, GameState, LocationKey, PlayerID, ServerIID};
use color_eyre::Result;
use toml::{Table, Value};
use walkdir::WalkDir;
use crate::game::cards::card::CardType::*;
use crate::game::cards::card_behaviour::Behaviour;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct SlotPosition {
    x : i32,
    y : i32,
    z : i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum CardType {
    Hero,
    Landscape {
        slots: Vec<SlotPosition>
    },
    Unit {
        attack: u32,
        health: u32,
        defense: u32,
    },
    Item,
    Command,
}

#[derive(Clone)]
pub struct CardData {
    pub owner: PlayerID,
    pub location: LocationKey,
    pub behaviours: Vec<Behaviour>,
    pub key: CardKey,
    pub card_id: String,
    pub instance_id: ServerIID,
    pub card_type: CardType,
    pub name: String,
    pub description: String,
    pub cost: u32,
    pub card_types: Vec<String>,
}

// Implement debug for card data
impl std::fmt::Debug for CardData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("card_data")
            //.field("behaviours", &self.behaviours)
            .field("owner", &self.owner)
            .field("card_id", &self.card_id)
            .field("instance_id", &self.instance_id)
            .field("card_type", &self.card_type)
            .field("name", &self.name)
            .field("description", &self.description)
            .field("cost", &self.cost)
            .field("card_types", &self.card_types)
            .finish()
    }
}

impl CardData {
    pub fn is_alive(&self, state: &GameState) -> bool {
        let graveyard1 = state.locations.get(state.board.side1.graveyard).unwrap();
        let graveyard2 = state.locations.get(state.board.side2.graveyard).unwrap();
        return graveyard1.contains(self.key) == false && graveyard2.contains(self.key) == false;
    }
}

pub struct CardRegistry {
    card_registry: HashMap<String, CardData>,
}

impl CardRegistry {
    pub fn from_directory(path: &str) -> Result<Self> {
        println!("Loading cards from {}", path);
        let mut card_registry = HashMap::new();
        for dir in WalkDir::new(path) {
            let dir = dir.unwrap();
            if dir.file_type().is_file() == false { continue; }
            let path = dir.path();
            if path.extension().unwrap() != "toml" { continue; }

            // Load text from path
            let text = fs::read_to_string(path).unwrap().parse::<Table>().unwrap();
            let name = text.get("name").unwrap().as_str().unwrap();
            let description = text.get("description").and_then(|x| x.as_str()).unwrap_or("");
            let cost = text.get("cost").unwrap().as_integer().unwrap();
            let types =
                if text.contains_key("types") { text.get("types").unwrap().as_array().unwrap().clone() } else { 
                    Vec::new()
                };
            let card_type = text.get("type").unwrap().as_str().unwrap();

            let file_name = path.file_name().unwrap().to_str().unwrap();
            
            let behaviours = text.get("behaviour");
            let behaviours = if behaviours.is_some() {
                let behaviours = behaviours.unwrap().as_array().unwrap();
                let behaviours = behaviours.iter().map(|b| {
                    let table = b.as_table();
                    Behaviour::from(table.unwrap().clone())
                }.unwrap()).collect::<Vec<_>>();
                behaviours
            } else {
                Vec::new()
            };
            
            let card = CardData {
                owner: PlayerID::Player1,
                location: LocationKey::default(),
                behaviours,
                card_id: file_name[..file_name.len() - 5].into(),
                instance_id: 0,
                key: CardKey::default(),
                name: name.to_string(),
                description: description.to_string(),
                cost: cost as u32,
                card_types: types.iter().map(|x| x.as_str().unwrap().to_string()).collect(),
                card_type: match card_type {
                    "landscape" => {
                        let slots = text.get("slots").unwrap().as_array().unwrap().clone();
                        let slots = slots.into_iter().map(|s| {
                            let pos = s.as_table().unwrap();
                            SlotPosition {
                                x: pos.get("x").unwrap().as_integer().unwrap() as i32,
                                y: pos.get("y").unwrap().as_integer().unwrap() as i32,
                                z: pos.get("z").unwrap().as_integer().unwrap() as i32,
                            }
                        }).collect();
                        Landscape { 
                            slots
                        }
                    },
                    "hero" => Hero,
                    "unit" => {
                        let attack = text.get("attack").unwrap().as_integer().unwrap();
                        let health = text.get("health").unwrap().as_integer().unwrap();
                        let defense = text.get("defense").unwrap().as_integer().unwrap();
                        Unit {
                            attack: attack as u32,
                            health: health as u32,
                            defense: defense as u32,
                        }
                    }
                    "item" => Item,
                    "command" => Command,
                    _ => return Err(eyre!("Invalid card type: {}", card_type)),
                },
            };

            println!("Loaded card: {}", card.card_id);
            card_registry.insert(card.card_id.clone(), card);
        }

        Ok(Self {
            card_registry
        })
    }

    pub fn create_card(&self, id: &str, iid: ServerIID, owner: PlayerID) -> Result<CardData> {
        let card = self.card_registry.get(id).ok_or_else(|| eyre!("Card not found: {}", id))?;
        let mut card = card.clone();
        card.instance_id = iid;
        card.owner = owner;
        Ok(card)
    }
}
