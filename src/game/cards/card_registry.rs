use std::collections::HashMap;
use std::fs;
use color_eyre::eyre::{ContextCompat, eyre};

use color_eyre::Result;
use walkdir::WalkDir;

use crate::game::cards::card_deserialization::{Card, CardCategory};
use crate::game::cards::card_instance::{CardInstance, UnitStats};
use crate::game::id_types::{CardInstanceId, LocationId, PlayerId};

pub struct CardRegistry {
    card_registry: HashMap<String, &'static Card>,
}

impl CardRegistry {
    pub fn from_directory(path: &str) -> Result<Self> {
        println!("Loading cards from {}", path);

        let mut registry: HashMap<String, &'static Card> = HashMap::new();

        for dir in WalkDir::new(path).into_iter().filter_map(|entry| entry.ok()) {
            if dir.path().is_file() == false {
                continue;
            }

            let id = dir.path().with_extension("").file_name().and_then(|name| name.to_str()).unwrap().to_string();
            println!("Loading card: {}", id);
            let mut card: Box<Card> = Box::new(toml::from_str(&fs::read_to_string(dir.path())?)?);
            card.id = id.clone();
            registry.insert(
                id,
                Box::leak(card)
            );
        }

        Ok(CardRegistry {
            card_registry: registry
        })
    }

    pub fn instance_card(&self, id: &str, instance_id: CardInstanceId, location: LocationId, owner: PlayerId) -> Result<CardInstance> {
        let card = self.card_registry.get(id).context(eyre!("Card not found: {}", id))?;

        let mut health = 0;
        let mut defense = 0;
        let mut attack = 0;

        match card.card_category {
            CardCategory::Hero => {}
            CardCategory::Landscape { .. } => {}
            CardCategory::Unit { health: h, attack: a, defense: d } => {
                health = h;
                attack = a;
                defense = d;
            }
            CardCategory::Item => {}
            CardCategory::Command => {}
        }

        Ok(CardInstance {
            card,
            owner,
            location,
            instance_id,
            behaviors: card.behaviors.clone(),
            cost: card.cost,
            stats: UnitStats { health, defense, attack },
            card_types: card.types.clone(),
        })
    }
}
