use color_eyre::eyre::eyre;
use crate::game::location::Location;

use color_eyre::Result;
use crate::game::id_types::{TokenInstanceId, LocationId, ServerInstanceId};

pub struct CardSlot {
    pub location_id: LocationId,
    pub card: Option<TokenInstanceId>,
}

impl CardSlot {
    pub fn new(location_id: LocationId) -> CardSlot {
        CardSlot {
            location_id,
            card : None,
        }
    }
}

impl Location for CardSlot {
    fn set_location_id(&mut self, lid: LocationId) {
        self.location_id = lid;
    }

    fn get_location_id(&self) -> LocationId {
        self.location_id
    }

    fn add_card(&mut self, card: TokenInstanceId) -> Result<()> {
        if self.card.is_some() { return Err(eyre!("Attempted to put card in card slot that is already populated")) }
        self.card = Some(card);
        Ok(())
    }

    fn remove_card(&mut self, card: TokenInstanceId) {
        if let Some(stored) = self.card && stored == stored {
            self.card = None;
        }
    }

    fn clear(&mut self) {
        self.card = None;
    }

    fn shuffle(&mut self) {
    }

    fn contains(&self, card: TokenInstanceId) -> bool { Some(card) == self.card }

    fn get_card(&self) -> Option<TokenInstanceId> {
        match self.card {
            None => None,
            Some(c) => Some(c),
        }
    }

    fn get_cards(&self) -> Vec<TokenInstanceId> {
        match self.card {
            None => Vec::new(),
            Some(c) => vec![c],
        }        
    }
}