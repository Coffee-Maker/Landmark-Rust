use color_eyre::Result;
use crate::game::id_types::{TokenInstanceId, LocationId, ServerInstanceId};

pub trait Location {
    fn set_location_id(&mut self, location_id: LocationId);
    fn get_location_id(&self) -> LocationId;

    fn add_token(&mut self, token: TokenInstanceId) -> Result<()>;
    fn remove_token(&mut self, token: TokenInstanceId);
    fn clear(&mut self);
    fn shuffle(&mut self);
    
    fn contains(&self, token: TokenInstanceId) -> bool;
    fn has_room(&self) -> bool;
    
    fn get_token(&self) -> Option<TokenInstanceId>;
    fn get_tokens(&self) -> Vec<TokenInstanceId>;
}