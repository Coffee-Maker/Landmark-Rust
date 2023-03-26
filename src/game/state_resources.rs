use std::collections::{HashMap, VecDeque};

use color_eyre::eyre::{Context, ContextCompat, eyre};
use color_eyre::Result;
use crate::CARD_REGISTRY;
use crate::game::animation_presets::AnimationPreset;

use crate::game::board::Board;
use crate::game::cards;
use crate::game::cards::card_deserialization::{CardBehavior, CardBehaviorAction, CardBehaviorTriggerWhenName, CardCategory};
use crate::game::cards::card_instance::CardInstance;
use crate::game::game_communicator::GameCommunicator;
use crate::game::id_types::{TokenInstanceId, location_ids, LocationId, PlayerId, ServerInstanceId};
use crate::game::instruction::InstructionToClient;
use crate::game::location::Location;
use crate::game::new_state_machine::{StateMachine, StateTransitionGroup};
use crate::game::player::Player;
use crate::game::trigger_context::{ContextValue, GameContext};

type ThreadSafeLocation = dyn Location + Send + Sync;

pub struct StateResources {
    pub locations: HashMap<LocationId, Box<ThreadSafeLocation>>,
    pub card_instances: HashMap<TokenInstanceId, CardInstance>,
    pub round: u32,
    pub player_1: Player,
    pub player_2: Player,
    pub current_turn: PlayerId,
    pub board: Board,
    location_counter: ServerInstanceId,
}

impl StateResources {
    pub fn new() -> Self {
        Self {
            locations: HashMap::new(),
            card_instances: HashMap::new(),
            round: 0,
            player_1: Player::new(PlayerId::Player1, location_ids::PLAYER_1_DECK, location_ids::PLAYER_1_HAND),
            player_2: Player::new(PlayerId::Player2, location_ids::PLAYER_2_DECK, location_ids::PLAYER_2_HAND),
            current_turn: if fastrand::bool() { PlayerId::Player1 } else { PlayerId::Player2 },
            board: Board::new(),
            location_counter: 0,
        }
    }

    pub fn insert_location(&mut self, mut location: Box<ThreadSafeLocation>) {
        self.locations.insert(location.get_location_id(), location);
    }

    pub async fn reset_game(&mut self, communicator: &mut GameCommunicator) -> Result<()> {
        for key in self.locations.iter().map(|(key, _)| key.clone()).collect::<Vec<_>>() {
            self.clear_location(key, communicator).await?;
        }
        Ok(())
    }

    pub async fn clear_location(&mut self, location: LocationId, communicator: &mut GameCommunicator) -> Result<()> {
        self.locations.get_mut(&location).context("Tried to clear a non existent location")?.clear();
        communicator.send_game_instruction(InstructionToClient::ClearLocation { location }).await
    }

    pub async fn move_card(&mut self, card_instance_id: TokenInstanceId, to: LocationId, move_owner: PlayerId, animation: Option<AnimationPreset>, communicator: &mut GameCommunicator) -> Result<()> {
        let mut card_instance = self.card_instances.get_mut(&card_instance_id).context("Card instance not found while attempting a move")?;
        let from = card_instance.location;
        let from_instance = self.locations.get_mut(&from).context("Tried to move card from a location that doesn't exist")?;
        from_instance.remove_card(card_instance_id);
        card_instance.location = to.clone();

        let to_instance = self
            .locations
            .get_mut(&to)
            .context("Tried to move a card to a location that doesn't exist")?;
        to_instance.add_card(card_instance_id)?;
        let to_id = to_instance.get_location_id();

        if let Some(animation) = animation {
            communicator.send_game_instruction(InstructionToClient::Animate {
                card: card_instance_id,
                location: to,
                duration: 0.5,
                preset: animation,
            }).await?;
        }

        communicator.send_game_instruction(InstructionToClient::MoveCard {
            card: card_instance_id,
            to: to_id
        }).await?;

        // Check if a unit changed landscapes
        let old_location = location_ids::identify_location(from)?;
        let new_location = location_ids::identify_location(to)?;

        if old_location.is_field() == false && new_location.is_field() {
            card_instance.hidden = false;
            if card_instance.hidden == false {
                communicator.send_game_instruction(InstructionToClient::Reveal { card: card_instance_id }).await?;
            }
        }

        Ok(())
    }

    pub async fn create_card(&mut self, id: &str, location: LocationId, owner: PlayerId, communicator: &mut GameCommunicator) -> Result<()> {
        let card_instance_id = TokenInstanceId(fastrand::u64(..));

        let loc = self.locations
            .get_mut(&location)
            .context("Tried to create a card to a location that does not exist")?;

        let mut card = match (*CARD_REGISTRY.lock().await).instance_card(&id, card_instance_id, location, owner) {
            Ok(card) => card,
            Err(e) => {
                eprintln!("{e}");
                return Err(e);
            }
        };
        card.instance_id = card_instance_id;
        card.location = location;

        communicator.send_game_instruction(InstructionToClient::CreateCard {
            card_data: card.clone(),
            instance_id: card_instance_id,
            player_id: owner,
            location_id: loc.get_location_id()
        }).await?;

        communicator.send_game_instruction(InstructionToClient::UpdateBehaviors { card_data: card.clone() }).await?;

        self.card_instances.insert(card_instance_id, card);
        loc.add_card(card_instance_id)?;

        Ok(())
    }

    pub async fn destroy_card(&mut self, card: TokenInstanceId, communicator: &mut GameCommunicator) -> Result<()> {
        let card_instance = self.card_instances.get(&card).unwrap();
        if matches!(card_instance.card.card_category, CardCategory::Hero { .. }) {
            communicator.send_game_instruction(InstructionToClient::EndGame { winner: card_instance.owner.opponent() }).await?;
            return Err(eyre!("Game has concluded"))
        }

        Ok(())
    }

    pub fn get_player(&self, id: PlayerId) -> &Player {
        match id {
            Player1 => &self.player_1,
            Player2 => &self.player_2,
        }
    }

    pub fn get_player_mut(&mut self, id: PlayerId) -> &mut Player {
        match id {
            Player1 => &mut self.player_1,
            Player2 => &mut self.player_2,
        }
    }
}