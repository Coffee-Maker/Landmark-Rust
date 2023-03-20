use color_eyre::eyre::{ContextCompat, eyre};
use color_eyre::Result;
use crate::game::board::Board;
use crate::game::cards::card_deserialization::CardCategory;

use crate::game::game_communicator::GameCommunicator;
use crate::game::id_types::{LocationId, PlayerId};
use crate::game::instruction::InstructionToClient;
use crate::game::state_resources::StateResources;

#[derive(Clone)]
pub struct Player {
    pub thaum: u32,
    pub id: PlayerId,
    pub deck: LocationId,
    pub hand: LocationId,
}

impl Player {
    pub fn new(id: PlayerId, deck: LocationId, hand: LocationId) -> Self {
        Self {
            id,
            thaum: 0,
            deck,
            hand,
        }
    }

    pub async fn set_thaum(&mut self, thaum: u32, communicator: &mut GameCommunicator) -> Result<()> {
        self.thaum = thaum;

        communicator.send_game_instruction(InstructionToClient::SetThaum {
            player_id: self.id,
            amount: thaum,
        }).await
    }

    pub async fn populate_deck(&self, data: &str, resources: &mut StateResources, communicator: &mut GameCommunicator) -> Result<()> {
        for split in data.split(',') {
            resources.create_card(split, self.deck, self.id, communicator).await?;
        }

        Ok(())
    }
    
    pub async fn prepare_deck(&mut self, resources: &mut StateResources, board: &Board, communicator: &mut GameCommunicator) -> Result<()> {
        // Find hero and landscape
        let heroes = resources.locations.get(&self.deck).context("ya nan")?.get_cards().iter()
            .filter_map(|&card_key| {
                if let Some(card_instance) = resources.card_instances.get(&card_key) && card_instance.card.card_category == CardCategory::Hero {
                    Some(card_key)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        match heroes.len() {
            1 => {
                let hero = *heroes.first().unwrap(); // We already checked that there is one item in the vector
                let hero_location = board.get_side(self.id).hero;
                resources.move_card(hero, hero_location, communicator).await?;
            }
            0 => return Err(eyre!("No hero found in deck")),
            _ => return Err(eyre!("Found more than one hero in deck")),
        }

        let landscapes = resources.locations.get(&self.deck).context("ya nan")?.get_cards().iter()
            .filter_map(|&card_key| {
                if let Some(card_instance) = resources.card_instances.get(&card_key) && matches!(card_instance.card.card_category, CardCategory::Landscape { .. }) {
                    Some(card_key)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        match landscapes.len() {
            1 => {
                let landscape = *landscapes.first().unwrap(); // We already checked that there is one item in the vector
                let landscape_location = board.get_side(self.id).landscape;
                resources.move_card(landscape, landscape_location, communicator).await?;
            }
            0 => return Err(eyre!("No hero found in deck")),
            _ => return Err(eyre!("Found more than one hero in deck")),
        }
        
        Ok(())
    }

    pub async fn draw_card(&self, resources: &mut StateResources, communicator: &mut GameCommunicator) -> Result<()> {
        let card = resources.locations.get(&self.deck).unwrap().get_card();

        match card {
            None => {
                todo!("lose instantly")
            }
            Some(card_key) => {
                resources.move_card(card_key, self.hand, communicator).await?;
                // Todo: Reimplement this
                //let mut context = TriggerContext::new();
                //context.add_card(state, card_key);
                //state.trigger_card_events(self.id, communicator, BehaviorTrigger::DrawCard, &context)?;
            }
        }

        Ok(())
    }
}