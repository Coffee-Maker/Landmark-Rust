use crate::game::cards::card::CardBehaviour;

pub struct DefaultCard {}

impl CardBehaviour for DefaultCard {}

impl DefaultCard {
    pub fn new() -> Self {
        Self {}
    }
}