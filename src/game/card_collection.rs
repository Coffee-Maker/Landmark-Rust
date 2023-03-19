use color_eyre::Result;

use crate::game::game_state::{CardKey, ServerInstanceId};
use crate::game::location::Location;

pub struct CardCollection {
    pub cards: Vec<CardKey>,
    pub lid: ServerInstanceId,
}

impl Location for CardCollection {
    fn set_location_id(&mut self, lid: ServerInstanceId) {
        self.lid = lid;
    }

    fn get_location_id(&self) -> ServerInstanceId {
        self.lid
    }

    fn add_card(&mut self, card: CardKey) -> Result<()> {
        self.cards.push(card);
        Ok(())
    }

    fn remove_card(&mut self, card: CardKey) {
        self.cards.retain(|c| &card != c);
    }

    fn clear(&mut self) {
        self.cards.clear()
    }

    fn contains(&self, card: CardKey) -> bool { self.cards.contains(&card) }

    fn get_card(&self) -> Option<CardKey> {
        self.cards.first().map(|o| o.to_owned())
    }

    fn get_cards(&self) -> Vec<CardKey> {
        self.cards.clone()
    }
}

impl CardCollection {
    pub fn new() -> Self {
        Self {
            cards: vec![],
            lid: 0,
        }
    }
}