use color_eyre::Result;
use color_eyre::eyre::{ContextCompat, eyre};
use crate::game::cards::card::CardCategory;
use crate::game::game_communicator::GameCommunicator;
use crate::game::game_state::{CardKey, GameState, LocationKey, PlayerId, ServerInstanceId};
use crate::game::game_state::PlayerId::{Player1, Player2};
use crate::game::instruction::Instruction;

pub struct Board {
    pub side_1: BoardSide,
    pub side_2: BoardSide,
}

impl Board {
    pub fn new() -> Board {
        Board {
            side_1: BoardSide::new(Player1, 100),
            side_2: BoardSide::new(Player2, 200),
        }
    }

    pub fn prepare_landscapes(&mut self, state: &mut GameState, communicator: &mut GameCommunicator) -> Result<()> {
        self.side_1.add_landscape_slots(state, communicator)
    }

    pub fn get_cards_in_play(&self, state: &GameState) -> Vec<CardKey> {
        let mut cards = Vec::new();
        cards.append(&mut self.side_1.get_cards_in_play(state).clone());
        cards.append(&mut self.side_2.get_cards_in_play(state).clone());

        cards
    }
    
    pub fn get_relevant_landscape(&self, state: &GameState, card: CardKey) -> Option<LocationKey> {
        let card = state.card_instances.get(card)?;

        return if self.side_1.field.contains(&card.location) {
            Some(self.side_1.landscape)
        } else if self.side_2.field.contains(&card.location) {
            Some(self.side_2.landscape)
        } else {
            None
        }
    }
}

pub struct BoardSide {
    pub hero: LocationKey,
    pub landscape: LocationKey,
    pub field: Vec<LocationKey>,
    pub graveyard: LocationKey,
    pub owner: PlayerId,
    location_id_offset: ServerInstanceId
}


impl BoardSide {
    pub fn new(owner: PlayerId, location_id_offset: ServerInstanceId) -> BoardSide {
        BoardSide {
            hero: LocationKey::default(),
            landscape: LocationKey::default(),
            field: Vec::new(),
            graveyard: LocationKey::default(),
            owner,
            location_id_offset,
        }
    }
    
    pub fn get_cards_in_play(&self, state: &GameState) -> Vec<CardKey> {
        let mut cards = Vec::new();
        cards.append(&mut state.locations.get(self.hero).unwrap().get_cards().clone());
        cards.append(&mut state.locations.get(self.landscape).unwrap().get_cards().clone());
        for loc in self.field.iter() {
            cards.append(&mut state.locations.get(*loc).unwrap().get_cards().clone());
        }
        cards.append(&mut state.locations.get(self.graveyard).unwrap().get_cards().clone());
        cards
    }

    pub fn add_landscape_slots(&mut self, state: &GameState, communicator: &mut GameCommunicator) -> Result<()> {
        let card_instance = state.card_instances.get(self.landscape).context(format!("Card in landscape slot for {} does not exist", self.owner))?;
        let landscape = &card_instance.card_category;
        match landscape {
            CardCategory::Landscape { slots } => {
                let mut i = 0;
                for _slot in slots {
                    communicator.queue.enqueue(Instruction::AddLandscapeSlot {
                        player: self.owner,
                        index: i,
                        location_id: self.location_id_offset,
                    });
                    i += 1;
                    self.location_id_offset += 1;
                }
                Ok(())
            }
            _ => { Err(eyre!("Given landscape was not a landscape... I blame Marc")) }
        }
    }
}