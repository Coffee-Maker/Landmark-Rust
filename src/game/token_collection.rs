use color_eyre::Result;

use crate::game::id_types::{TokenInstanceId, LocationId, ServerInstanceId};
use crate::game::location::Location;

pub struct TokenCollection {
    pub tokens: Vec<TokenInstanceId>,
    pub location_id: LocationId,
}

impl Location for TokenCollection {
    fn set_location_id(&mut self, lid: LocationId) {
        self.location_id = lid;
    }

    fn get_location_id(&self) -> LocationId {
        self.location_id
    }

    fn add_token(&mut self, token: TokenInstanceId) -> Result<()> {
        self.tokens.push(token);
        Ok(())
    }

    fn remove_token(&mut self, token: TokenInstanceId) {
        self.tokens.retain(|c| &token != c);
    }

    fn clear(&mut self) {
        self.tokens.clear()
    }

    fn shuffle(&mut self) {
        let mut new_tokens = Vec::new();
        while self.tokens.len() > 0{
            new_tokens.push(self.tokens.remove(fastrand::usize(0..self.tokens.len())));
        }
        self.tokens = new_tokens;
    }

    fn contains(&self, token: TokenInstanceId) -> bool { self.tokens.contains(&token) }

    fn get_token(&self) -> Option<TokenInstanceId> {
        self.tokens.first().map(|o| o.to_owned())
    }

    fn get_tokens(&self) -> Vec<TokenInstanceId> {
        self.tokens.clone()
    }
}

impl TokenCollection {
    pub fn new(location_id: LocationId) -> Self {
        Self {
            tokens: vec![],
            location_id,
        }
    }
}