use std::collections::HashMap;
use color_eyre::eyre::ContextCompat;
use crate::game::cards::default_card::DefaultCard;

use color_eyre::Result;
use crate::game::player::{Player, PlayerInstance};

pub type CardInstance<'a> = &'a CardData<'a>;
pub type MutCardInstance<'a> = &'a mut CardData<'a>;

#[derive(Clone, Copy)]
pub struct CardData<'a> {
    pub behaviour: &'a Box<dyn CardBehaviour>,
    pub owner: PlayerInstance<'a>,
    pub id: u32,
    pub iid: u32,
}

impl<'a> CardData<'a> {
    pub fn new(id: u32, iid: u32, behaviour: &'a Box<dyn CardBehaviour>,  owner: &'a Player<'a>) -> Self {
        Self {
            behaviour,
            owner,
            id,
            iid,
        }
    }
}

pub trait CardBehaviour {
    fn on_selected(&self, card: CardInstance) {
        
    }
    
    fn on_draw(&self, card: CardInstance) {
        
    }
    
    fn on_defeated(&self, card: CardInstance) {
        
    }
}

pub struct CardRegistry {
    card_registry: HashMap<u32, Box<dyn CardBehaviour>>,
}

impl CardRegistry {
    pub fn new() -> Self {
        let cards: [(u32, Box<dyn CardBehaviour>); 10] = [
            (0, Box::new(DefaultCard::new())),
            (1, Box::new(DefaultCard::new())),
            (2, Box::new(DefaultCard::new())),
            (3, Box::new(DefaultCard::new())),
            (4, Box::new(DefaultCard::new())),
            (5, Box::new(DefaultCard::new())),
            (6, Box::new(DefaultCard::new())),
            (7, Box::new(DefaultCard::new())),
            (8, Box::new(DefaultCard::new())),
            (9, Box::new(DefaultCard::new())),
        ];
        
        Self {
            card_registry: HashMap::from(cards),
        }
    }
    
    pub fn create_card(&self, id: u32, iid: u32, owner: PlayerInstance) -> Result<CardData> {
        let result = self.card_registry.get(&id).context(format!("Card index not found in registry: {id}"))?;
        let instance = CardData::new(id, iid, result, owner);
        Ok(instance)
    }
}