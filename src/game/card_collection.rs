use crate::game::game_state::{CardKey, ObjKey, ServerInstanceId};
use crate::game::location::Location;

pub struct CardCollection {
    pub cards: Vec<ObjKey>,
    pub lid: ServerInstanceId,
}

impl Location for CardCollection {
    fn set_lid(&mut self, lid: ServerInstanceId) {
        self.lid = lid;
    }

    fn get_lid(&self) -> ServerInstanceId {
        self.lid
    }

    fn add_card(&mut self, card: ObjKey) {
        self.cards.push(card);
    }

    fn remove_card(&mut self, card: ObjKey) {
        self.cards.retain(|c| &card != c);
    }

    fn clear(&mut self) {
        self.cards.clear()
    }

    fn contains(&self, card: CardKey) -> bool { self.cards.contains(&card) }

    fn get_card(&self) -> Option<ObjKey> {
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