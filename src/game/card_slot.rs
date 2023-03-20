use color_eyre::eyre::eyre;
use crate::game::game_state::{CardInstanceId, ServerInstanceId};
use crate::game::location::Location;

use color_eyre::Result;

pub struct CardSlot {
    pub lid: ServerInstanceId,
    pub card: Option<CardInstanceId>,
}

impl CardSlot {
    pub fn new() -> CardSlot {
        CardSlot {
            lid : 0,
            card : None,
        }
    }
}

impl Location for CardSlot {
    fn set_location_id(&mut self, lid: ServerInstanceId) {
        self.lid = lid;
    }

    fn get_location_id(&self) -> ServerInstanceId {
        self.lid
    }

    fn add_card(&mut self, _card: CardInstanceId) -> Result<()> {
        if self.card.is_some() { return Err(eyre!("Attempted to put card in card slot that is already populated")) }
        self.card = Some(_card);
        Ok(())
    }

    fn remove_card(&mut self, _card: CardInstanceId) {
        if self.card.is_some() && self.card.unwrap() == _card {
            self.card = None;
        }        
    }

    fn clear(&mut self) {
        self.card = None;
    }

    fn contains(&self, card: CardInstanceId) -> bool { Some(card) == self.card }

    fn get_card(&self) -> Option<CardInstanceId> {
        match self.card {
            None => None,
            Some(c) => Some(c),
        }
    }

    fn get_cards(&self) -> Vec<CardInstanceId> {
        match self.card {
            None => Vec::new(),
            Some(c) => vec![c],
        }        
    }
}