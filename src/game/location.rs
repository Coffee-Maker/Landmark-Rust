use crate::game::game_state::{CardInstanceId, LocationId, ServerInstanceId};

use color_eyre::Result;

pub trait Location {
    fn set_location_id(&mut self, location_id: LocationId);
    fn get_location_id(&self) -> LocationId;

    fn add_card(&mut self, card: CardInstanceId) -> Result<()>;
    fn remove_card(&mut self, card: CardInstanceId);
    fn clear(&mut self);
    
    fn contains(&self, card: CardInstanceId) -> bool;
    
    fn get_card(&self) -> Option<CardInstanceId>;
    fn get_cards(&self) -> Vec<CardInstanceId>;
}