use crate::game::game_state::{CardKey, ObjKey, ServerIID};
use crate::game::location::Location;

pub struct CardSlot {
    pub lid: ServerIID,
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
    fn set_lid(&mut self, lid: ServerIID) {
        self.lid = lid;
    }

    fn get_lid(&self) -> ServerIID {
        self.lid
    }

    fn add_card(&mut self, _card: ObjKey) {
        self.card = Some(_card);
        println!("Added card to slot {}", self.lid);
    }

    fn remove_card(&mut self, _card: ObjKey) {
        if self.card.is_some() && self.card.unwrap() == _card {
            self.card = None;
        }        
    }

    fn clear(&mut self) {
        self.card = None;
    }

    fn contains(&self, card: CardKey) -> bool { Some(card) == self.card }

    fn get_card(&self) -> Option<ObjKey> {
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
