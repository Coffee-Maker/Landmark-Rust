use std::collections::HashMap;
use std::fs;
use color_eyre::eyre::{ContextCompat, eyre};

use color_eyre::Result;
use walkdir::WalkDir;

use crate::game::cards::token_deserializer::{TokenData, TokenCategory};
use crate::game::cards::card_instance::{TokenInstance, UnitStats};
use crate::game::id_types::{TokenInstanceId, LocationId, PlayerId};

pub struct CardRegistry {
    pub card_registry: HashMap<String, &'static TokenData>,
}

impl CardRegistry {
    pub fn from_directory(path: &str) -> Result<Self> {
        println!("Loading tokens from {}", path);

        let mut registry: HashMap<String, &'static TokenData> = HashMap::new();

        for dir in WalkDir::new(path).into_iter().filter_map(|entry| entry.ok()) {
            if dir.path().is_file() == false {
                continue;
            }

            let id = dir.path().with_extension("").file_name().and_then(|name| name.to_str()).unwrap().to_string();
            println!("Loading token: {}", id);
            let mut token: Box<TokenData> = Box::new(toml::from_str(&fs::read_to_string(dir.path())?)?);
            token.id = id.clone();
            registry.insert(
                id,
                Box::leak(token)
            );
        }

        Ok(CardRegistry {
            card_registry: registry
        })
    }

    pub fn instance_card(&self, id: &str, instance_id: TokenInstanceId, location: LocationId, owner: PlayerId) -> Result<TokenInstance> {
        let token = self.card_registry.get(id).context(eyre!("Token not found: {}", id))?;

        let mut health = 0;
        let mut defense = 0;
        let mut attack = 0;

        match token.token_category {
            TokenCategory::Hero {health: h, defense: d} => {
                health = h;
                defense = d;
            }
            TokenCategory::Landscape { .. } => {}
            TokenCategory::Unit { health: h, attack: a, defense: d } => {
                health = h;
                attack = a;
                defense = d;
            }
            TokenCategory::Item => {}
            TokenCategory::Command => {}
        }

        Ok(TokenInstance {
            token_data: token,
            owner,
            location,
            instance_id,
            behaviors: token.behaviors.clone(),
            cost: token.cost,
            base_stats: UnitStats { health, defense, attack },
            current_stats: UnitStats { health, defense, attack },
            card_types: token.types.clone(),
            hidden: true
        })
    }

    pub fn get_data(&self, id: &str) -> Result<&TokenData> {
        Ok(*self.card_registry.get(id).context(eyre!("Card not found: {}", id))?)
    }
}
