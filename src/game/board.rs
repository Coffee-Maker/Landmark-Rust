use color_eyre::Result;
use color_eyre::eyre::{ContextCompat, eyre};
use crate::game::card_slot::CardSlot;
use crate::game::cards::card::CardCategory;
use crate::game::game_communicator::GameCommunicator;
use crate::game::game_state::{CardKey, LocationKey, PlayerId};
use crate::game::game_state::PlayerId::{Player1, Player2};
use crate::game::instruction::InstructionToClient;
use crate::game::state_resources::StateResources;

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

    pub async fn prepare_landscapes(&mut self, resources: &mut StateResources, communicator: &mut GameCommunicator) -> Result<()> {
        self.side_1.add_landscape_slots(resources, communicator, 100).await?;
        self.side_2.add_landscape_slots(resources, communicator, 200).await
    }

    pub fn get_cards_in_play(&self, resources: &StateResources) -> Vec<CardKey> {
        let mut cards = Vec::new();
        cards.append(&mut self.side_1.get_cards_in_play(resources).clone());
        cards.append(&mut self.side_2.get_cards_in_play(resources).clone());

        cards
    }
    
    pub fn get_relevant_landscape(&self, resources: &StateResources, card: CardKey) -> Option<LocationKey> {
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
    pub hero: LocationKey,
    pub landscape: LocationKey,
    pub field: Vec<LocationKey>,
    pub graveyard: LocationKey,
    pub owner: PlayerId,
}


impl BoardSide {
    pub fn new(owner: PlayerId) -> BoardSide {
        BoardSide {
            hero: LocationKey::default(),
            landscape: LocationKey::default(),
            field: Vec::new(),
            graveyard: LocationKey::default(),
            owner,
        }
    }
    
    pub fn get_cards_in_play(&self, resources: &StateResources) -> Vec<CardKey> {
        let mut cards = Vec::new();
        cards.append(&mut resources.locations.get(&self.hero).unwrap().get_cards().clone());
        cards.append(&mut resources.locations.get(&self.landscape).unwrap().get_cards().clone());
        for loc in self.field.iter() {
            cards.append(&mut resources.locations.get(loc).unwrap().get_cards().clone());
        }
        cards.append(&mut resources.locations.get(&self.graveyard).unwrap().get_cards().clone());
        cards
    }

    pub async fn add_landscape_slots(&mut self, resources: &mut StateResources, communicator: &mut GameCommunicator, location_id_offset: u64) -> Result<()> {
        let location = resources.locations.get(&self.landscape).context("Landscape location does not exist")?;
        let card_instance = resources.card_instances.get(&location.get_card().context("Landscape card was not provided")?).context(format!("Card in landscape slot for {} does not exist", self.owner))?;
        let landscape = card_instance.card_category.clone();

        match landscape {
            CardCategory::Landscape { slots } => {
                let mut i = 0 as u64;
                for _slot in slots {
                    let location_id = location_id_offset + i;
                    communicator.send_game_instruction(InstructionToClient::AddLandscapeSlot {
                        player_id: self.owner,
                        index: i,
                        location_id,
                    }).await?;
                    i += 1;

                    let new_loc = CardSlot::new();
                    resources.add_location(location_id, Box::new(new_loc));
                    self.field.push(location_id);
                }
                Ok(())
            }
            _ => { Err(eyre!("Given landscape was not a landscape... I blame Marc (?????????????)")) }
        }
    }
}