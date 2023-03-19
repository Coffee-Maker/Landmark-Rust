use std::collections::HashMap;
use std::fs;

use color_eyre::eyre::{ContextCompat, eyre};
use color_eyre::Result;
use serde::Deserialize;
use toml::Table;
use walkdir::WalkDir;
use crate::game::board::Board;

use crate::game::cards::card::CardCategory::*;
use crate::game::cards::card_behavior::Behavior;
use crate::game::game_state::{CardKey, LocationKey, PlayerId, ServerInstanceId};
use crate::game::state_resources::StateResources;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Deserialize)]
pub struct SlotPosition {
    x: i32,
    y: i32,
    z: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum CardCategory {
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
    pub owner: PlayerId,
    pub location: LocationKey,
    pub behaviors: Vec<Behavior>,
    pub key: CardKey,
    pub card_id: String,
    pub instance_id: ServerInstanceId,
    pub card_category: CardCategory,
    pub name: String,
    pub description: String,
    pub cost: u32,
    pub card_types: Vec<String>,
}

struct CardFile {
    name: String,
    description: String,
    cost: u64,
    cart_types: Vec<String>,
    card_category: CardCategory
}

// Implement debug for card data
impl std::fmt::Debug for CardData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("card_data")
            //.field("behaviours", &self.behaviours)
            .field("owner", &self.owner)
            .field("card_id", &self.card_id)
            .field("instance_id", &self.instance_id)
            .field("card_type", &self.card_category)
            .field("name", &self.name)
            .field("description", &self.description)
            .field("cost", &self.cost)
            .field("card_types", &self.card_types)
            .finish()
    }
}

impl CardData {
    pub fn is_alive(&self, resources: &StateResources, board: &Board) -> bool {
        let graveyard1 = resources.locations.get(&board.side_1.graveyard).unwrap();
        let graveyard2 = resources.locations.get(&board.side_2.graveyard).unwrap();
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

        for dir in WalkDir::new(path).into_iter().filter_map(|entry| entry.ok()) {
            if dir.file_type().is_file() == false { continue; }
            let path = dir.path();
            if path.extension().and_then(|extension| if extension == "toml" { Some(()) } else { None }).is_none() { continue; }

            // Load text from path
            let text = fs::read_to_string(path)?.parse::<Table>()?;
            let name = text.get("name").context("Card name not found")?.as_str().context("Card name is not a string")?;
            let description = text.get("description").and_then(|x| x.as_str()).unwrap_or("");
            let cost = text.get("cost").unwrap().as_integer().unwrap();
            let types =
                if text.contains_key("types") {
                    text.get("types").unwrap().as_array().unwrap().clone()
                } else {
                    Vec::new()
                };
            let card_category = text.get("type").unwrap().as_str().unwrap();

            let file_name = path.file_name().unwrap().to_str().unwrap();
            
            let behaviours = text.get("behaviour");
            let behaviours = if behaviours.is_some() {
                let behaviours = behaviours.unwrap().as_array().unwrap();
                let behaviours = behaviours.iter().map(|b| {
                    let table = b.as_table();
                    Behavior::from(table.unwrap().clone())
                }.unwrap()).collect::<Vec<_>>();

                behaviours
            } else {
                Vec::new()
            };
            
            let card = CardData {
                owner: PlayerId::Player1,
                location: LocationKey::default(),
                behaviors: behaviours,
                card_id: file_name[..file_name.len() - 5].into(),
                instance_id: 0,
                key: CardKey::default(),
                name: name.to_string(),
                description: description.to_string(),
                cost: cost as u32,
                card_types: types.iter().map(|x| x.as_str().unwrap().to_string()).collect(),
                card_category: match card_category {
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
                    _ => return Err(eyre!("Invalid card type: {}", card_category)),
                },
            };

            println!("Loaded card: {}", card.card_id);
            card_registry.insert(card.card_id.clone(), card);
        }

        Ok(Self {
            card_registry
        })
    }

    pub fn instance_card(&self, id: &str, instance_id: ServerInstanceId, owner: PlayerId) -> Result<CardData> {
        let card = self.card_registry.get(id).ok_or_else(|| eyre!("Card not found: {}", id))?;
        let mut card = card.clone();
        card.instance_id = instance_id;
        card.owner = owner;
        Ok(card)
    }
}
