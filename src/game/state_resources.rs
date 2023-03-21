use std::collections::HashMap;

use color_eyre::eyre::ContextCompat;
use color_eyre::Result;

use crate::game::board::Board;
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
        let mut card_behavior_trigger_contexts = CardBehaviorTriggerQueue::new();

        let mut context = CardBehaviorContext::new(move_owner);
        context.insert("card_instance", ContextValue::CardInstance(card_instance_id));

        card_behavior_trigger_contexts.push_back((CardBehaviorTriggerWhenName::WillBeMoved, context.clone()));
        card_behavior_trigger_contexts.push_back((CardBehaviorTriggerWhenName::WillEnterLandscape, context.clone()));

        Ok(card_behavior_trigger_contexts)
    }

    pub async fn move_card(&mut self, card_instance_id: CardInstanceId, to: LocationId, move_owner: PlayerId, communicator: &mut GameCommunicator) -> Result<CardBehaviorTriggerQueue> {
        let mut card_instance = self.card_instances.get_mut(&card_instance_id).context("Card instance not found while attempting a move")?;
        let from = card_instance.location;
        let from_instance = self.locations.get_mut(&from).context("Tried to move card from a location that doesn't exist")?;
        from_instance.remove_card(card_instance_id);
        card_instance.location = to.clone();
        //let owner = card_instance.owner.clone();

        let to_instance = self
            .locations
            .get_mut(&to)
            .context("Tried to move a card to a location that doesn't exist")?;
        to_instance.add_card(card_instance_id)?;
        let to_id = to_instance.get_location_id();

        communicator.send_game_instruction(InstructionToClient::MoveCard {
            card: card_instance_id,
            to: to_id
        }).await?;

        // Send card behavior triggers
        let mut trigger_queue = CardBehaviorTriggerQueue::new();

        let mut context= CardBehaviorContext::new(move_owner);
        context.insert("card_instance", ContextValue::CardInstance(card_instance_id));
        trigger_queue.push_back((CardBehaviorTriggerWhenName::HasBeenMoved, context));

        // Check if a unit changed landscapes
        let from_location_identity = location_ids::identify_location(from)?;
        let to_location_identity = location_ids::identify_location(to)?;

        let from_field_1 = from_location_identity == location_ids::LocationIdentity::Player1Field;
        let from_field_2 = from_location_identity == location_ids::LocationIdentity::Player2Field;
        let to_field_1 = to_location_identity == location_ids::LocationIdentity::Player1Field;
        let to_field_2 = to_location_identity == location_ids::LocationIdentity::Player2Field;

        if (from_field_1 != to_field_1) && (from_field_2 != to_field_2) && ((from_field_1 && to_field_2) || (from_field_2 && to_field_1)) { // It checks if the card just entered a landscape
            let mut context = CardBehaviorContext::new(move_owner);
            context.insert("card_instance", ContextValue::CardInstance(card_instance_id));
            trigger_queue.push_back((CardBehaviorTriggerWhenName::HasLeftLandscape, context));

            let mut context = CardBehaviorContext::new(move_owner);
            context.insert("card_instance", ContextValue::CardInstance(card_instance_id));
            trigger_queue.push_back((CardBehaviorTriggerWhenName::HasEnteredLandscape, context));
        }

        Ok(trigger_queue)
    }

    pub async fn create_card(&mut self, id: &str, location: LocationId, owner: PlayerId, communicator: &mut GameCommunicator) -> Result<()> {
        let card_instance_id = CardInstanceId(fastrand::u64(..));

        let loc = self.locations
            .get_mut(&location)
            .context("Tried to create a card to a location that does not exist")?;

        let mut card = match (*CARD_REGISTRY.lock().await).instance_card(&id, card_instance_id, location, owner) {
            Ok(card) => card,
            Err(e) => {
                eprintln!("{e}");
                return Ok(());
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

        self.card_instances.insert(card_instance_id, card);
        loc.add_card(card_instance_id)?;

        // Todo: Reimplement this
        // if self.board.get_relevant_landscape(&self.card_instances, card_key).is_some() {
        //     let mut context = TriggerContext::new();
        //     context.add_card(self, card_key);
        //     self.trigger_card_events(owner, communicator, BehaviorTrigger::Summon, &context)?;
        //     self.trigger_card_events(owner, communicator, BehaviorTrigger::EnterLandscape, &context)?;
        // }

        Ok(())
    }

    pub async fn destroy_card(&mut self, board: &mut Board, card: CardInstanceId, communicator: &mut GameCommunicator) -> Result<()> {
        let card_instance = self.card_instances.get(&card).unwrap();

        communicator.send_game_instruction( InstructionToClient::MoveCard {
            card: self.card_instances.get(&card).context("Card to be destroyed not found")?.instance_id,
            to: self.locations.get(&board.get_side(card_instance.owner).graveyard).context("Graveyard not found")?.get_location_id()
        }).await
    }
}