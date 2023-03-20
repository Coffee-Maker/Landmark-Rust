use std::collections::HashMap;
use std::fs;
use color_eyre::eyre::{ContextCompat, eyre};

use color_eyre::Result;
use walkdir::WalkDir;

use crate::game::cards::card_deserialization::Card;
use crate::game::cards::card_instance::CardInstance;
use crate::game::game_state::{CardInstanceId, LocationId, PlayerId};

pub struct CardRegistry {
    card_registry: HashMap<String, &'static Card>,
}

impl CardRegistry {
    pub fn from_directory(path: &str) -> Result<Self> {
        println!("Loading cards from {}", path);

        let mut registry: HashMap<String, &'static Card> = HashMap::new();

        for dir in WalkDir::new(path).into_iter().filter_map(|entry| entry.ok()) {
            registry.insert(
                dir.path().with_extension("").file_name().and_then(|name| name.to_str()).unwrap().to_string(),
                Box::leak(Box::new(toml::from_str(&fs::read_to_string(dir.path())?)?))
            );
        }

        Ok(CardRegistry {
            card_registry: registry
        })
    }

    pub fn instance_card(&self, id: &str, instance_id: CardInstanceId, location: LocationId, owner: PlayerId) -> Result<CardInstance> {
        let card = self.card_registry.get(id).context(eyre!("Card not found: {}", id))?;

        Ok(CardInstance {
            card,
            owner,
            location,
            instance_id,
            behaviors: card.behaviors.clone(),
            cost: card.cost,
            card_types: card.types.clone(),
        })
    }
}
