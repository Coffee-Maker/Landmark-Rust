use color_eyre::Result;
use crate::game::id_types::{TokenInstanceId, LocationId, ServerInstanceId};

pub trait Location {
    fn set_location_id(&mut self, location_id: LocationId);
    fn get_location_id(&self) -> LocationId;

    fn add_card(&mut self, card: TokenInstanceId) -> Result<()>;
    fn remove_card(&mut self, card: TokenInstanceId);
    fn clear(&mut self);
    fn shuffle(&mut self);
    
    fn contains(&self, card: TokenInstanceId) -> bool;
    
    fn get_card(&self) -> Option<TokenInstanceId>;
    fn get_cards(&self) -> Vec<TokenInstanceId>;
}