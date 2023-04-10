use std::collections::HashMap;
use color_eyre::Result;
use color_eyre::eyre::{ContextCompat, eyre};
use crate::game::tokens::token_deserializer::{TokenCategory, SlotPosition};
use crate::game::game_communicator::GameCommunicator;
use crate::game::id_types::PlayerId::{Player1, Player2};
use crate::game::instruction::InstructionToClient;
use crate::game::state_resources::StateResources;

use crate::game::game_service;
use crate::game::id_types::{TokenInstanceId, location_ids, LocationId, PlayerId};
use crate::game::locations::token_slot::TokenSlot;

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
        BoardSide::add_landscape_slots(PlayerId::Player1, resources, communicator).await?;
        BoardSide::add_landscape_slots(PlayerId::Player2, resources, communicator).await
    }

    pub fn get_tokens_in_play(&self, resources: &StateResources) -> Vec<TokenInstanceId> {
        let mut tokens = Vec::new();
        tokens.append(&mut self.side_1.get_tokens_in_play(resources).clone());
        tokens.append(&mut self.side_2.get_tokens_in_play(resources).clone());

        tokens
    }
    
    pub fn get_relevant_landscape(&self, resources: &StateResources, token: TokenInstanceId) -> Option<LocationId> {
        let token = resources.token_instances.get(&token)?;

        return if self.side_1.field.contains(&token.location) {
            Some(self.side_1.landscape)
        } else if self.side_2.field.contains(&token.location) {
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
    
    pub fn get_tokens_in_play(&self, resources: &StateResources) -> Vec<TokenInstanceId> {
        let mut tokens = Vec::new();
        tokens.append(&mut resources.locations.get(&self.hero).unwrap().get_tokens().clone());
        tokens.append(&mut resources.locations.get(&self.landscape).unwrap().get_tokens().clone());
        for loc in self.field.iter() {
            tokens.append(&mut resources.locations.get(loc).unwrap().get_tokens().clone());
        }
        tokens.append(&mut resources.locations.get(&self.graveyard).unwrap().get_tokens().clone());

        tokens
    }

    pub async fn add_landscape_slots(owner: PlayerId, resources: &mut StateResources, communicator: &mut GameCommunicator) -> Result<()> {
        let side = resources.board.get_side_mut(owner);
        let location = resources.locations.get(&side.landscape).context("Landscape location does not exist")?;
        let token_instance = resources.token_instances.get(&location.get_token().context("Landscape token was not provided")?).context(format!("Token in landscape slot for {} does not exist", owner))?;
        let landscape = &token_instance.token_data.token_category;

        match landscape {
            TokenCategory::Landscape { slots } => {
                let mut i = 0 as u64;

                for slot in slots {
                    let location_id = location_ids::player_field_location_id(owner, i);

                    communicator.send_game_instruction(InstructionToClient::AddLandscapeSlot {
                        player_id: owner,
                        index: i,
                        location_id,
                    }).await?;
                    i += 1;

                    let new_loc = TokenSlot::new(location_id);
                    side.field.push(location_id);
                    side.field_slot_positions.push(*slot);
                    resources.locations.insert(location_id, Box::new(new_loc));
                }
                Ok(())
            }
            _ => { Err(eyre!("Given landscape was not a landscape... I blame Marc (?????????????)")) }
        }
    }
}