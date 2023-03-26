use std::collections::HashMap;
use color_eyre::Result;
use color_eyre::eyre::{ContextCompat, eyre};
use crate::game::card_slot::CardSlot;
use crate::game::cards::card_deserialization::{CardCategory, SlotPosition};
use crate::game::game_communicator::GameCommunicator;
use crate::game::id_types::PlayerId::{Player1, Player2};
use crate::game::instruction::InstructionToClient;
use crate::game::state_resources::StateResources;

use crate::game::game_state;
use crate::game::id_types::{TokenInstanceId, location_ids, LocationId, PlayerId};

#[derive(Clone)]
pub struct Board {
    pub side_1: BoardSide,
    pub side_2: BoardSide,
}

impl Board {
    pub fn new() -> Board {
        Board {
            side_1: BoardSide::new(Player1),
            side_2: BoardSide::new(Player2),
        }
    }

    pub async fn prepare_landscapes(resources: &mut StateResources, communicator: &mut GameCommunicator) -> Result<()> {
        BoardSide::add_landscape_slots(resources, communicator).await?;
        BoardSide::add_landscape_slots(resources, communicator).await
    }

    pub fn get_cards_in_play(&self, resources: &StateResources) -> Vec<TokenInstanceId> {
        let mut cards = Vec::new();
        cards.append(&mut self.side_1.get_cards_in_play(resources).clone());
        cards.append(&mut self.side_2.get_cards_in_play(resources).clone());

        cards
    }
    
    pub fn get_relevant_landscape(&self, resources: &StateResources, card: TokenInstanceId) -> Option<LocationId> {
        let card = resources.card_instances.get(&card)?;

        return if self.side_1.field.contains(&card.location) {
            Some(self.side_1.landscape)
        } else if self.side_2.field.contains(&card.location) {
            Some(self.side_2.landscape)
        } else {
            None
        }
    }

    pub fn get_side_mut(&mut self, id: PlayerId) -> &mut BoardSide {
        match id {
            Player1 => &mut self.side_1,
            Player2 => &mut self.side_2,
        }
    }

    pub fn get_side(&self, id: PlayerId) -> &BoardSide {
        match id {
            Player1 => &self.side_1,
            Player2 => &self.side_2,
        }
    }
}

#[derive(Clone)]
pub struct BoardSide {
    pub hero: LocationId,
    pub landscape: LocationId,
    pub field: Vec<LocationId>,
    pub field_slot_positions: Vec<SlotPosition>,
    pub graveyard: LocationId,
    pub owner: PlayerId,
}


impl BoardSide {
    pub fn new(owner: PlayerId) -> BoardSide {
        BoardSide {
            hero: location_ids::player_hero_location_id(owner),
            landscape: location_ids::player_landscape_location_id(owner),
            field: Vec::new(),
            field_slot_positions: Vec::new(),
            graveyard: location_ids::player_graveyard_location_id(owner),
            owner,
        }
    }
    
    pub fn get_cards_in_play(&self, resources: &StateResources) -> Vec<TokenInstanceId> {
        let mut cards = Vec::new();
        cards.append(&mut resources.locations.get(&self.hero).unwrap().get_cards().clone());
        cards.append(&mut resources.locations.get(&self.landscape).unwrap().get_cards().clone());
        for loc in self.field.iter() {
            cards.append(&mut resources.locations.get(loc).unwrap().get_cards().clone());
        }
        cards.append(&mut resources.locations.get(&self.graveyard).unwrap().get_cards().clone());

        cards
    }

    pub async fn add_landscape_slots(owner: PlayerId, resources: &mut StateResources, communicator: &mut GameCommunicator) -> Result<()> {
        let side = resources.board.get_side_mut(owner);
        let location = resources.locations.get(&side.landscape).context("Landscape location does not exist")?;
        let card_instance = resources.card_instances.get(&location.get_card().context("Landscape card was not provided")?).context(format!("Card in landscape slot for {} does not exist", owner))?;
        let landscape = &card_instance.card.card_category;

        match landscape {
            CardCategory::Landscape { slots } => {
                let mut i = 0 as u64;

                for slot in slots {
                    let location_id = location_ids::player_field_location_id(owner, i);

                    communicator.send_game_instruction(InstructionToClient::AddLandscapeSlot {
                        player_id: owner,
                        index: i,
                        location_id,
                    }).await?;
                    i += 1;

                    let new_loc = CardSlot::new(location_id);
                    side.field.push(location_id);
                    side.field_slot_positions.push(*slot);
                    resources.insert_location(Box::new(new_loc));
                }
                Ok(())
            }
            _ => { Err(eyre!("Given landscape was not a landscape... I blame Marc (?????????????)")) }
        }
    }
}