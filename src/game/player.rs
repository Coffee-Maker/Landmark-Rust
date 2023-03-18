use crate::game::game_state::{GameState, LocationKey, PlayerId};
use crate::game::instruction::{Instruction, InstructionQueue};
use color_eyre::Result;
use color_eyre::eyre::{ContextCompat, eyre};
use crate::game::cards::card::CardCategory;
use crate::game::cards::card_behavior::BehaviorTrigger;
use crate::game::game_communicator::GameCommunicator;
use crate::game::trigger_context::TriggerContext;

pub struct Player {
    pub thaum: u32,
    pub id: PlayerId,
    pub deck: LocationKey,
    pub hand: LocationKey,
}

impl Player {
    pub fn new(id: PlayerId, deck: LocationKey, hand: LocationKey) -> Self {
        Self {
            id,
            thaum: 0,
            deck,
            hand,
        }
    }

    pub fn set_thaum(&self, thaum: u32, queue: &mut InstructionQueue) {
        queue.enqueue(Instruction::SetThaum {
            player: self.id,
            amount: thaum,
        });
    }

    pub fn populate_deck(&self, data: &str, queue: &mut InstructionQueue) -> Result<()> {
        let splits = data.split(',');
        for split in splits {
            queue.enqueue(Instruction::CreateCard {
                id: split.into(),
                iid: fastrand::u64(..),
                location: self.deck,
                player: self.id,
            });
        }
        Ok(())
    }
    
    pub fn prepare_deck(&self, state: &GameState, queue: &mut InstructionQueue) -> Result<()> {
        let side = state.get_side(self.id);

        // Find hero and landscape
        let mut found_hero = false;
        let mut found_landscape = false;

        let heroes = state.locations.get(self.deck).context("ya nan")?.get_cards().iter()
            .filter_map(|&card_key| {
                if let Some(card) = state.card_instances.get(card_key) && card.card_category == CardCategory::Hero {
                    Some(card_key)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        match heroes.len() {
            1 => {
                queue.enqueue(Instruction::MoveCard {
                    card: *heroes.first().unwrap(),
                    to: side.hero,
                });
            }
            0 => return Err(eyre!("No hero found in deck")),
            _ => return Err(eyre!("Found more than one hero in deck")),
        }

        let landscapes = state.locations.get(self.deck).context("ya nan")?.get_cards().iter()
            .filter_map(|&card_key| {
                if let Some(card) = state.card_instances.get(card_key) && matches!(card.card_category, CardCategory::Landscape { .. }) {
                    Some(card_key)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        match landscapes.len() {
            1 => {
                queue.enqueue(Instruction::MoveCard {
                    card: *landscapes.first().unwrap(),
                    to: side.hero,
                });
            }
            0 => return Err(eyre!("No hero found in deck")),
            _ => return Err(eyre!("Found more than one hero in deck")),
        }
        
        Ok(())
    }

    pub fn draw_card(&self, state: &GameState, communicator: &mut GameCommunicator) -> Result<()> {
        let card = state.locations.get(self.deck).unwrap().get_card();

        match card {
            None => {
                todo!("lose instantly")
            }
            Some(card_key) => {
                communicator.queue.enqueue(Instruction::DrawCard { player: self.id });
                let mut context = TriggerContext::new();
                context.add_card(state, card_key);
                state.trigger_card_events(self.id, communicator, BehaviorTrigger::DrawCard, &context)?;
            }
        }

        Ok(())
    }
}