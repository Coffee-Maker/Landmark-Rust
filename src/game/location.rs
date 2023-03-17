use crate::game::game_state::{CardKey, ServerIID};

pub trait Location {
    fn set_lid(&mut self, lid: ServerIID);
    fn get_lid(&self) -> ServerIID;

    fn add_card(&mut self, card: CardKey);
    fn remove_card(&mut self, card: CardKey);
    fn clear(&mut self);
    
    fn contains(&self, card: CardKey) -> bool;
    
    fn get_card(&self) -> Option<CardKey>;
    fn get_cards(&self) -> Vec<CardKey>;
}
