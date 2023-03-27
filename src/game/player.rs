use color_eyre::eyre::{ContextCompat, eyre};
use color_eyre::Result;
use crate::game::animation_presets::AnimationPreset;
use crate::game::board::Board;
use crate::game::cards::token_deserializer::TokenCategory;

use crate::game::game_communicator::GameCommunicator;
use crate::game::id_types::{TokenInstanceId, LocationId, PlayerId};
use crate::game::instruction::InstructionToClient;
use crate::game::state_resources::StateResources;

#[derive(Clone)]
pub struct Player {
    pub thaum: u32,
    pub id: PlayerId,
    pub deck: LocationId,
    pub hand: LocationId,
    pub hero: TokenInstanceId,
    pub landscape: TokenInstanceId,
}

impl Player {
    pub fn new(id: PlayerId, deck: LocationId, hand: LocationId) -> Self {
        Self {
            id,
            thaum: 0,
            deck,
            hand,
            hero: TokenInstanceId(0),
            landscape: TokenInstanceId(0)
        }
    }

    pub async fn set_thaum(player_id: PlayerId, resources: &mut StateResources, thaum: u32, communicator: &mut GameCommunicator) -> Result<()> {
        resources.get_player_mut(player_id).thaum = thaum;

        communicator.send_game_instruction(InstructionToClient::SetThaum {
            player_id,
            amount: thaum,
        }).await
    }

    pub async fn spend_thaum(player_id: PlayerId, resources: &mut StateResources, amount: u32, communicator: &mut GameCommunicator) -> Result<()> {
        resources.get_player_mut(player_id).thaum -= amount;
        let thaum = resources.get_player_mut(player_id).thaum;

        communicator.send_game_instruction(InstructionToClient::SetThaum {
            player_id,
            amount: thaum,
        }).await
    }

    pub async fn populate_deck(player_id: PlayerId, data: &str, resources: &mut StateResources, communicator: &mut GameCommunicator) -> Result<()> {
        let player = resources.get_player(player_id);
        let player_deck = player.deck;
        for split in data.split(',') {
            resources.create_token(split, player_deck, player_id, communicator).await?;
        }

        Ok(())
    }
    
    pub async fn prepare_deck(player_id: PlayerId, resources: &mut StateResources, communicator: &mut GameCommunicator) -> Result<()> {
        let player_deck = resources.get_player(player_id).deck;

        // Find hero and landscape
        let heroes = resources.locations.get(&player_deck).context("ya nan")?.get_cards().iter()
            .filter_map(|&card_key| {
                if let Some(card_instance) = resources.token_instances.get(&card_key) && matches!(card_instance.token_data.token_category, TokenCategory::Hero { .. }){
                    Some(card_key)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        match heroes.len() {
            1 => {
                let hero = *heroes.first().unwrap(); // We already checked that there is one item in the vector
                resources.get_player_mut(player_id).hero = hero;
                let hero_location = resources.board.get_side(player_id).hero;
                resources.move_token(hero, hero_location, None, communicator).await?;
            }
            0 => return Err(eyre!("No hero found in deck")),
            _ => return Err(eyre!("Found more than one hero in deck")),
        }

        let landscapes = resources.locations.get(&player_deck).context("ya nan")?.get_cards().iter()
            .filter_map(|&card_key| {
                if let Some(card_instance) = resources.token_instances.get(&card_key) && matches!(card_instance.token_data.token_category, TokenCategory::Landscape { .. }) {
                    Some(card_key)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        match landscapes.len() {
            1 => {
                let landscape = *landscapes.first().unwrap(); // We already checked that there is one item in the vector
                resources.get_player_mut(player_id).landscape = landscape;
                let landscape_location = resources.board.get_side(player_id).landscape;
                resources.move_token(landscape, landscape_location, None, communicator).await?;
            }
            0 => return Err(eyre!("No hero found in deck")),
            _ => return Err(eyre!("Found more than one hero in deck")),
        }

        resources.locations.get_mut(&player_deck).context("Deck was not found")?.shuffle();

        Ok(())
    }

    pub async fn draw_card(player_id: PlayerId, resources: &mut StateResources, communicator: &mut GameCommunicator) -> Result<()> {
        let player_deck = resources.get_player(player_id).deck;
        let player_hand = resources.get_player(player_id).hand;
        let card = resources.locations.get(&player_deck).unwrap().get_card();

        match card {
            None => {
                communicator.send_game_instruction(InstructionToClient::EndGame { winner: player_id.opponent() }).await?;
                return Err(eyre!("Ran out of tokens, game concluded"))
            }
            Some(card_key) => {
                resources.move_token(card_key, player_hand, None, communicator).await?;
            }
        }

        Ok(())
    }
}