use crate::game::game_state::{CardKey, ServerInstanceId};

use color_eyre::Result;

pub trait Location {
    fn set_location_id(&mut self, lid: ServerInstanceId);
    fn get_location_id(&self) -> ServerInstanceId;

    fn add_card(&mut self, card: CardKey) -> Result<()>;
    fn remove_card(&mut self, card: CardKey);
    fn clear(&mut self);
    
    fn contains(&self, card: CardKey) -> bool;
    
    fn get_card(&self) -> Option<CardKey>;
    fn get_cards(&self) -> Vec<CardKey>;
}