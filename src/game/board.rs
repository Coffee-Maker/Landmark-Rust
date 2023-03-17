use crate::game::game_state::{CardKey, GameState, LocationKey};

pub struct Board {
    pub side1: BoardSide,
    pub side2: BoardSide,
}

impl Board {
    pub fn new() -> Board {
        Board {
            side1: BoardSide::new(),
            side2: BoardSide::new(),
        }
    }
    
    pub fn get_cards_in_play(&self, state: &GameState) -> Vec<CardKey> {
        let mut cards = Vec::new();
        cards.append(&mut self.side1.get_cards_in_play(state).clone());
        cards.append(&mut self.side2.get_cards_in_play(state).clone());
        cards
    }
    
    pub fn get_relevant_landscape(&self, state: &GameState, card: CardKey) -> Option<LocationKey> {
        let card = state.card_instances.get(card).unwrap();
        if self.side1.field.contains(&card.location) {
            return Some(self.side1.landscape);
        } else if self.side2.field.contains(&card.location) {
            return Some(self.side2.landscape);
        }
        None
    }
}

pub struct BoardSide {
    pub hero: LocationKey,
    pub landscape: LocationKey,
    pub field: Vec<LocationKey>,
    pub graveyard: LocationKey,
}

impl BoardSide {
    pub fn new() -> BoardSide {
        BoardSide {
            hero: LocationKey::default(),
            landscape: LocationKey::default(),
            field: Vec::new(),
            graveyard: LocationKey::default(),
        }
    }
    
    pub fn get_cards_in_play(&self, state: &GameState) -> Vec<CardKey> {
        let mut cards = Vec::new();
        cards.append(&mut state.locations.get(self.hero).unwrap().get_cards().clone());
        cards.append(&mut state.locations.get(self.landscape).unwrap().get_cards().clone());
        for loc in self.field.iter() {
            cards.append(&mut state.locations.get(*loc).unwrap().get_cards().clone());
        }
        cards.append(&mut state.locations.get(self.graveyard).unwrap().get_cards().clone());
        cards
    }
}