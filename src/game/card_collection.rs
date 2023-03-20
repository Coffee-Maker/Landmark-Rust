use color_eyre::Result;

use crate::game::id_types::{CardInstanceId, LocationId, ServerInstanceId};
use crate::game::location::Location;

pub struct CardCollection {
    pub cards: Vec<CardInstanceId>,
    pub location_id: LocationId,
}

impl Location for CardCollection {
    fn set_location_id(&mut self, lid: LocationId) {
        self.location_id = lid;
    }

    fn get_location_id(&self) -> LocationId {
        self.location_id
    }

    fn add_card(&mut self, card: CardInstanceId) -> Result<()> {
        self.cards.push(card);
        Ok(())
    }

    fn remove_card(&mut self, card: CardInstanceId) {
        self.cards.retain(|c| &card != c);
    }

    fn clear(&mut self) {
        self.cards.clear()
    }

    fn contains(&self, card: CardInstanceId) -> bool { self.cards.contains(&card) }

    fn get_card(&self) -> Option<CardInstanceId> {
        self.cards.first().map(|o| o.to_owned())
    }

    fn get_cards(&self) -> Vec<CardInstanceId> {
        self.cards.clone()
    }
}

impl CardCollection {
    pub fn new(location_id: LocationId) -> Self {
        Self {
            cards: vec![],
            location_id,
        }
    }
}