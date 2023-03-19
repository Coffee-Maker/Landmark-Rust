use std::collections::HashMap;
use color_eyre::eyre::ContextCompat;
use crate::game::board::Board;
use crate::game::cards::card::CardData;
use crate::game::game_communicator::GameCommunicator;
use crate::game::game_state::{CARD_REGISTRY, CardKey, LocationKey, PlayerId, ServerInstanceId};
use crate::game::instruction::InstructionToClient;

use color_eyre::Result;
use crate::game::location::Location;

type ThreadSafeLocation = dyn Location + Send + Sync;

pub struct StateResources {
    pub locations: HashMap<LocationKey, Box<ThreadSafeLocation>>,
    pub card_instances: HashMap<CardKey, CardData>,
}

impl StateResources {
    pub fn new() -> Self {
        Self {
            locations: HashMap::new(),
            card_instances: HashMap::new(),
        }
    }

    pub fn add_location(&mut self, location_instance_id: ServerInstanceId, mut location: Box<ThreadSafeLocation>) -> LocationKey {
        location.set_location_id(location_instance_id);
        self.locations.insert(location_instance_id, location);
        location_instance_id
    }

    pub async fn reset_game(&mut self, communicator: &mut GameCommunicator) -> Result<()> {
        for key in self.locations.iter().map(|(key, _)| key.clone()).collect::<Vec<_>>() {
            self.clear_location(communicator, key).await?;
        }

        Ok(())
    }

    pub async fn clear_location(&mut self, communicator: &mut GameCommunicator, location: LocationKey) -> Result<()> {
        self.locations.get_mut(&location).context("Tried to clear a non existent location")?.clear();
        communicator.send_game_instruction(InstructionToClient::ClearLocation { location }).await
    }

    pub async fn move_card(&mut self, card: CardKey, to: LocationKey, communicator: &mut GameCommunicator) -> Result<()> {
        let mut card_instance = self.card_instances.get_mut(&card).context("Card instance not found while attempting a move")?;
        let from = card_instance.location;
        let from_instance = self.locations.get_mut(&from).context("Tried to move card from a location that doesn't exist")?;
        from_instance.remove_card(card);
        card_instance.location = to.clone();
        //let owner = card_instance.owner.clone();

        let to_instance = self
            .locations
            .get_mut(&to)
            .context("Tried to move a card to a location that doesn't exist")?;
        to_instance.add_card(card)?;
        let to_id = to_instance.get_location_id();

        communicator.send_game_instruction(InstructionToClient::MoveCard {
            card: self.card_instances.get(&card).context("Card to be moved not found")?.instance_id,
            to: to_id
        }).await?;

        // let from_side_1 = self.board.side_1.field.contains(&from);
        // let from_side_2 = self.board.side_2.field.contains(&from);
        // let to_side_1 = self.board.side_1.field.contains(&to);
        // let to_side_2 = self.board.side_2.field.contains(&to);

        // Todo: Reimplement this
        // if (to_side_1 || to_side_2) && (from_side_1 != to_side_1 || from_side_2 != to_side_2) { // It checks if the card just entered a landscape
        //     let mut context = TriggerContext::new();
        //     context.add_card(self, card);
        //     self.trigger_card_events(owner, communicator, BehaviorTrigger::EnterLandscape, &TriggerContext::new())?;
        // }

        Ok(())
    }

    pub async fn create_card(&mut self, id: &str, location: LocationKey, owner: PlayerId, communicator: &mut GameCommunicator) -> Result<()> {
        let card_key = fastrand::u64(..);

        let loc = self.locations
            .get_mut(&location)
            .context("Tried to create a card to a location that does not exist")?;

        let mut card = match (*CARD_REGISTRY.lock().await).instance_card(&id, card_key, owner) {
            Ok(card) => card,
            Err(e) => {
                eprintln!("{e}");
                return Ok(());
            }
        };
        card.key = card_key;
        card.location = location;

        communicator.send_game_instruction(InstructionToClient::CreateCard {
            card_data: card.clone(),
            instance_id: card_key,
            player_id: owner,
            location_id: loc.get_location_id()
        }).await?;

        self.card_instances.insert(card_key, card);
        loc.add_card(card_key)?;

        // Todo: Reimplement this
        // if self.board.get_relevant_landscape(&self.card_instances, card_key).is_some() {
        //     let mut context = TriggerContext::new();
        //     context.add_card(self, card_key);
        //     self.trigger_card_events(owner, communicator, BehaviorTrigger::Summon, &context)?;
        //     self.trigger_card_events(owner, communicator, BehaviorTrigger::EnterLandscape, &context)?;
        // }

        Ok(())
    }

    pub async fn destroy_card(&mut self, board: &mut Board, card: CardKey, communicator: &mut GameCommunicator) -> Result<()> {
        let card_instance = self.card_instances.get(&card).unwrap();

        communicator.send_game_instruction( InstructionToClient::MoveCard {
            card: self.card_instances.get(&card).context("Card to be destroyed not found")?.instance_id,
            to: self.locations.get(&board.get_side(card_instance.owner).graveyard).context("Graveyard not found")?.get_location_id()
        }).await
    }
}