use color_eyre::eyre::eyre;

use color_eyre::Result;
use crate::game::id_types::{TokenInstanceId, LocationId, ServerInstanceId};
use crate::game::locations::location::Location;

#[derive(Clone, Copy, Debug)]
pub struct TokenSlot {
    pub location_id: LocationId,
    pub token: Option<TokenInstanceId>,
}

impl TokenSlot {
    pub fn new(location_id: LocationId) -> TokenSlot {
        TokenSlot {
            location_id,
            token : None,
        }
    }
}

impl Location for TokenSlot {
    fn set_location_id(&mut self, lid: LocationId) {
        self.location_id = lid;
    }

    fn get_location_id(&self) -> LocationId {
        self.location_id
    }

    fn add_token(&mut self, token: TokenInstanceId) -> Result<()> {
        if self.token.is_some() { return Err(eyre!("Attempted to put token in token slot that is already populated")) }
        self.token = Some(token);
        Ok(())
    }

    fn remove_token(&mut self, token: TokenInstanceId) {
        if let Some(stored) = self.token && stored == stored {
            self.token = None;
        }
    }

    fn clear(&mut self) {
        self.token = None;
    }

    fn shuffle(&mut self) {
    }

    fn contains(&self, token: TokenInstanceId) -> bool { Some(token) == self.token }

    fn has_room(&self) -> bool {
        self.token.is_none()
    }

    fn get_token(&self) -> Option<TokenInstanceId> {
        match self.token {
            None => None,
            Some(c) => Some(c),
        }
    }

    fn get_tokens(&self) -> Vec<TokenInstanceId> {
        match self.token {
            None => Vec::new(),
            Some(c) => vec![c],
        }        
    }
}