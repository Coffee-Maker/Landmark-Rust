use std::collections::HashMap;

use color_eyre::eyre::{Context, ContextCompat};
use color_eyre::Result;
use crate::game::animation_presets::AnimationPreset;

use crate::game::board::Board;
use crate::game::cards;
use crate::game::cards::card_deserialization::{CardBehavior, CardBehaviorAction, CardBehaviorTriggerWhenName};
use crate::game::cards::card_instance::CardInstance;
use crate::game::game_communicator::GameCommunicator;
use crate::game::game_state::{CARD_REGISTRY, CardBehaviorTriggerQueue};
use crate::game::id_types::{CardInstanceId, location_ids, LocationId, PlayerId, ServerInstanceId};
use crate::game::instruction::InstructionToClient;
use crate::game::location::Location;
use crate::game::trigger_context::{CardBehaviorContext, ContextValue};

type ThreadSafeLocation = dyn Location + Send + Sync;

pub struct StateResources {
    pub locations: HashMap<LocationId, Box<ThreadSafeLocation>>,
    pub card_instances: HashMap<CardInstanceId, CardInstance>,
}

impl StateResources {
    pub fn new() -> Self {
        Self {
            locations: HashMap::new(),
            card_instances: HashMap::new(),
        }
    }

    pub fn insert_location(&mut self, mut location: Box<ThreadSafeLocation>) {
        self.locations.insert(location.get_location_id(), location);
    }

    pub async fn reset_game(&mut self, communicator: &mut GameCommunicator) -> Result<()> {
        for key in self.locations.iter().map(|(key, _)| key.clone()).collect::<Vec<_>>() {
            self.clear_location(communicator, key).await?;
        }

        Ok(())
    }

    pub async fn clear_location(&mut self, communicator: &mut GameCommunicator, location: LocationId) -> Result<()> {
        self.locations.get_mut(&location).context("Tried to clear a non existent location")?.clear();
        communicator.send_game_instruction(InstructionToClient::ClearLocation { location }).await
    }

    pub async fn pre_move_card(&self, card_instance_id: CardInstanceId, to: LocationId, move_owner: PlayerId, communicator: &mut GameCommunicator) -> Result<CardBehaviorTriggerQueue> {
        let mut trigger_queue = CardBehaviorTriggerQueue::new();

        let mut context = CardBehaviorContext::new(move_owner);
        context.insert("card_instance", ContextValue::CardInstance(card_instance_id));

        trigger_queue.push_back((CardBehaviorTriggerWhenName::WillBeMoved, context.clone()));

        // Check if a unit changed landscapes
        let old_location = location_ids::identify_location(
            self.card_instances.get(&card_instance_id).context("Tried to move card that does not exist")?.location).context("Tried to move card from a location that is not identifiable")?;
        let new_location = location_ids::identify_location(to).context("Tried to move card to non-identifiable location")?;

        if old_location.is_field() && old_location != new_location { // It checks if the card just entered a landscape
            trigger_queue.push_back((CardBehaviorTriggerWhenName::WillLeaveLandscape, context.clone()));

            if new_location.is_field() {
                trigger_queue.push_back((CardBehaviorTriggerWhenName::WillEnterLandscape, context.clone()));
            }
        }

        if old_location.is_field() == false && new_location.is_field() {
            trigger_queue.push_back((CardBehaviorTriggerWhenName::WillBeSummoned, context.clone()));
            trigger_queue.push_back((CardBehaviorTriggerWhenName::WillEnterLandscape, context.clone()));
        }

        Ok(trigger_queue)
    }

    pub async fn move_card(&mut self, card_instance_id: CardInstanceId, to: LocationId, move_owner: PlayerId, animation: Option<AnimationPreset>, communicator: &mut GameCommunicator) -> Result<CardBehaviorTriggerQueue> {
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

        // Send card behavior triggers
        let mut trigger_queue = CardBehaviorTriggerQueue::new();

        let mut context= CardBehaviorContext::new(move_owner);
        context.insert("card_instance", ContextValue::CardInstance(card_instance_id));
        trigger_queue.push_back((CardBehaviorTriggerWhenName::HasBeenMoved, context.clone()));

        // Check if a unit changed landscapes
        let old_location = location_ids::identify_location(from)?;
        let new_location = location_ids::identify_location(to)?;

        if old_location.is_field() && old_location != new_location { // It checks if the card just entered a landscape
            trigger_queue.push_back((CardBehaviorTriggerWhenName::HasLeftLandscape, context.clone()));

            if new_location.is_field() {
                trigger_queue.push_back((CardBehaviorTriggerWhenName::HasEnteredLandscape, context.clone()));
            }
        }

        if old_location.is_field() == false && new_location.is_field() {
            trigger_queue.push_back((CardBehaviorTriggerWhenName::HasBeenSummoned, context.clone()));
            let mut context = CardBehaviorContext::new(move_owner);
            context.insert("card_instance", ContextValue::CardInstance(card_instance_id));
            trigger_queue.push_back((CardBehaviorTriggerWhenName::HasEnteredLandscape, context));
        }

        Ok(trigger_queue)
    }

    pub async fn create_card(&mut self, id: &str, location: LocationId, owner: PlayerId, communicator: &mut GameCommunicator) -> Result<CardBehaviorTriggerQueue> {
        let card_instance_id = CardInstanceId(fastrand::u64(..));

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

        let mut queue = CardBehaviorTriggerQueue::new();
        if location_ids::identify_location(location).context("Tried to create card to non-identifiable location")?.is_field() {
            let mut context = CardBehaviorContext::new(owner);
            context.insert("card_instance", ContextValue::CardInstance(card_instance_id));
            queue.push_back((CardBehaviorTriggerWhenName::HasBeenSummoned, context.clone()));
            queue.push_back((CardBehaviorTriggerWhenName::HasEnteredLandscape, context));
        }

        Ok(queue)
    }

    pub async fn destroy_card(&mut self, board: &mut Board, card: CardInstanceId, communicator: &mut GameCommunicator) -> Result<CardBehaviorTriggerQueue> {
        let card_instance = self.card_instances.get(&card).unwrap();
        self.move_card(card, location_ids::player_graveyard_location_id(card_instance.owner), card_instance.owner, Some(AnimationPreset::EaseInOut), communicator).await
    }
}