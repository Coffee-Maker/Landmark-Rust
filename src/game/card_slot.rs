use color_eyre::eyre::eyre;
use crate::game::game_state::{CardKey, ServerInstanceId};
use crate::game::location::Location;

use color_eyre::Result;

pub struct CardSlot {
    pub lid: ServerInstanceId,
    pub card: Option<CardKey>,
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

    fn add_card(&mut self, _card: CardKey) -> Result<()> {
        if self.card.is_some() { return Err(eyre!("Attempted to put card in card slot that is already populated")) }
        self.card = Some(_card);
        Ok(())
    }

    fn remove_card(&mut self, _card: CardKey) {
        if self.card.is_some() && self.card.unwrap() == _card {
            self.card = None;
        }        
    }

    fn clear(&mut self) {
        self.card = None;
    }

    fn contains(&self, card: CardKey) -> bool { Some(card) == self.card }

    fn get_card(&self) -> Option<CardKey> {
        match self.card {
            None => None,
            Some(c) => Some(c),
        }
    }

    fn get_cards(&self) -> Vec<CardKey> {
        match self.card {
            None => Vec::new(),
            Some(c) => vec![c],
        }        
    }
}