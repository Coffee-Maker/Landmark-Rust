use std::fmt::{Display, Formatter};
use std::num::ParseIntError;
use std::str::FromStr;

use color_eyre::eyre;
use color_eyre::eyre::{ContextCompat, eyre};
use eyre::Result;
use once_cell::sync::Lazy;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::WebSocketStream;

use crate::game::board::Board;
use crate::game::card_collection::CardCollection;
use crate::game::card_slot::CardSlot;
use crate::game::cards::card_registry::CardRegistry;
use crate::game::game_communicator::GameCommunicator;
use crate::game::id_types::{CardInstanceId, LocationId, PlayerId, ServerInstanceId};
use crate::game::id_types::PlayerId::{Player1, Player2};
use crate::game::instruction::InstructionToClient;
use crate::game::player::Player;
use crate::game::state_resources::StateResources;
use crate::game::tag::get_tag;
use crate::game::trigger_context::CardBehaviorTriggerContext;

pub async fn game_service(websocket: WebSocketStream<TcpStream>) -> Result<()> {
    println!("Starting game service");

    let mut communicator = GameCommunicator::new(websocket);
    let mut game_state = GameState::new();

    loop {
        let msg = communicator.read_message().await?;

        println!("Received message: {}", msg);

        let message = msg.into_text().unwrap();

        let [instruction, data] = message.split('|').collect::<Vec<_>>()[..] else {
            println!("Could not execute invalid instruction.");
            continue;
        };

        let result = match instruction {
            "start_game" => game_state.start_game(data, &mut communicator).await,
            "move_card" => game_state.player_moved_card(data, &mut communicator).await,
            "pass_turn" => game_state.player_pass_turn(data, &mut communicator).await,
            _ => Err(eyre!("Unknown instruction: {}", instruction)),
        };

        // if result.is_ok() {
        //     result = communicator.process_instructions(self).await;
        // }

        match result {
            Ok(_) => { }
            Err(e) => {
                communicator.send_error(&e.to_string()).await?;
            }
        }
    }
}

pub static CARD_REGISTRY: Lazy<Mutex<CardRegistry>> = Lazy::new(|| {
    Mutex::new(CardRegistry::from_directory("data/cards").unwrap())
});

pub struct GameState {
    pub current_turn: PlayerId,
    player_1: Player,
    player_2: Player,
    pub resources: StateResources,
    pub board: Board,

    location_counter: ServerInstanceId,
}

pub mod location_ids {
    use crate::game::id_types::PlayerId::Player1;
    use crate::game::id_types::{LocationId, PlayerId};
    use crate::game::player::Player;

    pub const PLAYER_1_DECK: LocationId = LocationId(100);
    pub const PLAYER_1_HAND: LocationId = LocationId(101);
    pub const PLAYER_1_HERO: LocationId = LocationId(102);
    pub const PLAYER_1_LANDSCAPE: LocationId = LocationId(103);
    pub const PLAYER_1_GRAVEYARD: LocationId = LocationId(104);
    pub const PLAYER_2_DECK: LocationId = LocationId(200);
    pub const PLAYER_2_HAND: LocationId = LocationId(201);
    pub const PLAYER_2_HERO: LocationId = LocationId(202);
    pub const PLAYER_2_LANDSCAPE: LocationId = LocationId(203);
    pub const PLAYER_2_GRAVEYARD: LocationId = LocationId(204);

    pub fn player_deck_location_id(player: PlayerId, index: u64) -> LocationId {
        if player == Player1 { PLAYER_1_DECK } else { PLAYER_2_DECK }
    }

    pub fn player_hand_location_id(player: PlayerId, index: u64) -> LocationId {
        if player == Player1 { PLAYER_1_HAND } else { PLAYER_2_HAND }
    }

    pub fn player_hero_location_id(player: PlayerId) -> LocationId {
        if player == Player1 { PLAYER_1_HERO } else { PLAYER_2_HERO }
    }

    pub fn player_landscape_location_id(player: PlayerId) -> LocationId {
        if player == Player1 { PLAYER_1_LANDSCAPE } else { PLAYER_2_LANDSCAPE }
    }

    pub fn player_graveyard_location_id(player: PlayerId) -> LocationId {
        if player == Player1 { PLAYER_1_GRAVEYARD } else { PLAYER_2_GRAVEYARD }
    }

    pub fn player_field_location_id(player: PlayerId, index: u64) -> LocationId {
        LocationId(if player == Player1 { 1000 } else { 2000 } + index)
    }
}

impl GameState {
    pub fn new() -> Self {
        Self {
            current_turn: Player1,
            player_1: Player::new(Player1, location_ids::PLAYER_1_DECK, location_ids::PLAYER_1_HAND),
            player_2: Player::new(Player2, location_ids::PLAYER_2_DECK, location_ids::PLAYER_2_HAND),
            resources: StateResources::new(),
            board: Board::new(),
            location_counter: 100,
        }
    }

    pub async fn start_game(mut self: &mut Self, data: &str, communicator: &mut GameCommunicator) -> Result<()> {
        self.resources.insert_location(Box::new(CardCollection::new(location_ids::PLAYER_1_DECK)));
        self.resources.insert_location(Box::new(CardCollection::new(location_ids::PLAYER_1_HAND)));
        self.resources.insert_location(Box::new(CardCollection::new(location_ids::PLAYER_2_DECK)));
        self.resources.insert_location(Box::new(CardCollection::new(location_ids::PLAYER_2_HAND)));
        self.resources.insert_location(Box::new(CardSlot::new(location_ids::PLAYER_1_HERO)));
        self.resources.insert_location(Box::new(CardSlot::new(location_ids::PLAYER_2_HERO)));
        self.resources.insert_location(Box::new(CardSlot::new(location_ids::PLAYER_1_LANDSCAPE)));
        self.resources.insert_location(Box::new(CardSlot::new(location_ids::PLAYER_2_LANDSCAPE)));
        self.resources.insert_location(Box::new(CardCollection::new(location_ids::PLAYER_1_GRAVEYARD)));
        self.resources.insert_location(Box::new(CardCollection::new(location_ids::PLAYER_2_GRAVEYARD)));
        self.resources.reset_game(communicator).await?;

        // Populate decks
        let deck_1_string = get_tag("deck1", data)?;
        let deck_2_string = get_tag("deck2", data)?;
        self.player_1.populate_deck(&deck_1_string[..], &mut self.resources, communicator).await?;
        self.player_2.populate_deck(&deck_2_string[..], &mut self.resources, communicator).await?;

        self.player_1.set_thaum( 0, communicator).await?;
        self.player_1.prepare_deck(&mut self.resources, &self.board, communicator).await?;

        self.player_2.set_thaum( 0, communicator).await?;
        self.player_2.prepare_deck(&mut self.resources, &self.board, communicator).await?;

        self.board.prepare_landscapes(&mut self.resources, communicator).await?;

        for _ in 0..5 {
            self.player_1.draw_card(&mut self.resources, communicator).await?;
            self.player_2.draw_card(&mut self.resources, communicator).await?;
        }

        // Set random turn
        self.set_current_turn(if fastrand::bool() { Player1 } else { Player2 }, communicator).await?;

        Ok(())
    }

    pub async fn player_moved_card(&mut self, data: &str, communicator: &mut GameCommunicator) -> Result<()> {
        let card_id = get_tag("card", data)?.parse::<CardInstanceId>()?;
        let target_location_id = get_tag("location", data)?.parse::<LocationId>()?;

        let card = self.resources.card_instances.get(&card_id).context("Unable to find card")?;

        if card.location == target_location_id {
            return Ok(());
        }

        if card.owner != self.current_turn {
            communicator.send_error("Can't play card out of turn").await?;
            communicator.send_game_instruction( InstructionToClient::MoveCard { card: card.instance_id, to: card.location }).await?;
            return Ok(());
        }

        if card.location != self.get_player(card.owner).hand {
            communicator.send_error("Can't play card from this location").await?;
            communicator.send_game_instruction( InstructionToClient::MoveCard { card: card_id, to: card.location }).await?;
            return Ok(());
        }

        if self.board.get_side(card.owner).field.contains(&target_location_id) == false {
            communicator.send_error("Can't play card to this location").await?;
            communicator.send_game_instruction( InstructionToClient::MoveCard { card: card_id, to: card.location }).await?;
            return Ok(());
        }

        self.resources.move_card(card_id, target_location_id, communicator).await?;

        Ok(())
    }

    pub async fn player_pass_turn(&mut self, data: &str, communicator: &mut GameCommunicator) -> Result<()> {
        self.current_turn = if self.current_turn == PlayerId::Player1 { PlayerId::Player2 } else { PlayerId::Player1 };
        communicator.send_game_instruction( InstructionToClient::PassTurn { player_id: self.current_turn }).await?;

        Ok(())
    }

    pub async fn set_current_turn(&mut self, player_id: PlayerId, communicator: &mut GameCommunicator) -> Result<()> {
        self.current_turn = player_id;
        communicator.send_game_instruction(InstructionToClient::PassTurn { player_id }).await
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

    // Todo: Reimplement this
    // pub fn trigger_card_events(&mut self, resources: &mut StateResources, trigger_owner: PlayerId, communicator: &mut GameCommunicator, trigger: BehaviorTrigger, context: &TriggerContext) -> Result<()> {
    //     let mut locations = vec![
    //         self.board.side_1.hero, self.board.side_1.landscape, self.board.side_1.graveyard,
    //         self.board.side_1.hero, self.board.side_1.landscape, self.board.side_2.graveyard,
    //     ];
    //
    //     locations.append(&mut self.board.side_1.field.clone());
    //     locations.append(&mut self.board.side_2.field.clone());
    //
    //     for location in locations {
    //         let location = resources.locations.get(&location).unwrap();
    //
    //         for key in location.get_cards() {
    //             let card = resources.card_instances.get(&key).unwrap();
    //
    //             for behavior in &card.behaviors {
    //                 // Todo: Reimplement this
    //                 // behavior.trigger(trigger, trigger_owner, context, self, communicator, key)?;
    //             }
    //         }
    //     }
    //
    //     Ok(())
    // }
}
