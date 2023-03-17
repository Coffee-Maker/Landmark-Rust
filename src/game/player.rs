use crate::game::game_state::{CardKey, GameState, LocationKey, ObjKey, PlayerID, ServerIID};
use crate::game::instruction::{Instruction, InstructionQueue};
use crate::game::location::Location;
use color_eyre::Result;
use std::collections::VecDeque;
use std::rc::Rc;
use color_eyre::eyre::ContextCompat;
use crate::game::cards::card::CardType;
use crate::game::game_communicator::GameCommunicator;

pub struct Player {
    pub thaum: u32,
    pub id: PlayerID,
    pub deck: LocationKey,
    pub hand: LocationKey,
}

impl Player {
    pub fn new(id: PlayerID, deck: LocationKey, hand: LocationKey) -> Self {
        Self {
            id,
            thaum: 0,
            deck,
            hand,
        }
    }

    pub fn set_thaum(&self, thaum: u32, queue: &mut VecDeque<Instruction>) {
        queue.push_back(Instruction::SetThaum {
            player: self.id,
            amount: thaum,
        });
    }

    pub fn populate_deck(&self, data: &str, queue: InstructionQueue) -> Result<()> {
        let splits = data.split(',');
        for split in splits {
            queue.push_back(Instruction::CreateCard {
                id: split.into(),
                iid: fastrand::u64(..),
                location: self.deck,
                player: self.id,
            });
        }
        Ok(())
    }
    
    pub fn prepare_deck(&self, state: &GameState, queue: InstructionQueue) -> Result<()> {
        let side = state.get_side(self.id);
        // Find hero and landscape
        let mut found_hero = false;
        let mut found_landscape = false;
        
        for card in state.locations.get(self.deck).unwrap().get_cards() {
            let card_instance = state.card_instances.get(card).context("Card instance not found")?;
            match &card_instance.card_type {
                CardType::Hero => {
                    if found_hero {
                        return Err(color_eyre::eyre::eyre!("Found more than one hero in deck"));
                    }

                    queue.push_back(Instruction::MoveCard {
                        card,
                        to: side.hero,
                    });
                    found_hero = true;
                },
                CardType::Landscape { slots: _slots } => {
                    if found_landscape {
                        return Err(color_eyre::eyre::eyre!("Found more than one landscape in deck"));
                    }

                    queue.push_back(Instruction::MoveCard {
                        card,
                        to: side.landscape,
                    });
                    found_landscape = true;
                },
                _ => {}
            }
        }
        
        if !found_hero {
            return Err(color_eyre::eyre::eyre!("No hero found in deck"));
        }
        
        if !found_landscape {
            return Err(color_eyre::eyre::eyre!("No landscape found in deck"));
        }
        
        Ok(())
    }

    pub fn draw_card(&self, state: &GameState, comm: &mut GameCommunicator) -> Result<()> {
        state.draw_card(self.id, comm)
    }
}

pub struct CardCollection {
    pub cards: Vec<ObjKey>,
    pub lid: ServerIID,
}

impl Location for CardCollection {
    fn set_lid(&mut self, lid: ServerIID) {
        self.lid = lid;
    }

    fn get_lid(&self) -> ServerIID {
        self.lid
    }

    fn add_card(&mut self, card: ObjKey) {
        self.cards.push(card);
    }

    fn remove_card(&mut self, card: ObjKey) {
        self.cards.retain(|c| card != c.to_owned());
    }

    fn clear(&mut self) {
        self.cards.clear()
    }

    fn contains(&self, card: CardKey) -> bool { self.cards.contains(&card) }

    fn get_card(&self) -> Option<ObjKey> {
        self.cards.first().map(|o| o.to_owned())
    }

    fn get_cards(&self) -> Vec<CardKey> {
        self.cards.clone()
    }
}


impl CardCollection {
    pub fn new() -> Self {
        Self {
            cards: vec![],
            lid: 0,
        }
    }
}