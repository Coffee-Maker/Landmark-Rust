use std::fmt::{Display, Formatter};

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
use crate::game::cards::card::CardRegistry;
use crate::game::cards::card_behavior::BehaviorTrigger;
use crate::game::game_communicator::GameCommunicator;
use crate::game::game_state::PlayerId::{Player1, Player2};
use crate::game::instruction::InstructionToClient;
use crate::game::player::Player;
use crate::game::state_resources::StateResources;
use crate::game::tag::get_tag;
use crate::game::trigger_context::TriggerContext;

pub async fn game_service(websocket: WebSocketStream<TcpStream>) -> Result<()> {
    println!("Starting game service");

    let mut communicator = GameCommunicator::new(websocket);
    let mut game_state = GameState::new();

    loop {
        let msg = communicator.read_message().await?;

        println!("Received message: {}", msg);

        let message = msg.into_text().unwrap();

        let [instruction, data] = &message.split('|').collect::<Vec<_>>()[..] else {
            println!("Could not execute invalid instruction.");
            continue;
        };

        let result = match *instruction {
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

pub type ServerInstanceId = u64;
pub type LocationKey = ServerInstanceId;
pub type CardKey = ServerInstanceId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlayerId {
    Player1 = 0,
    Player2 = 1,
}

impl Display for PlayerId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Player1 => write!(f, "Player 1"),
            Player2 => write!(f, "Player 2"),
        }
    }
}

pub struct GameState {
    pub current_turn: PlayerId,
    player_1: Player,
    player_2: Player,
    pub resources: StateResources,
    pub board: Board,

    location_counter: ServerInstanceId,
}

impl GameState {
    pub fn new() -> Self {
        Self {
            current_turn: Player1,
            player_1: Player::new(Player1, LocationKey::default(), LocationKey::default()),
            player_2: Player::new(Player2, LocationKey::default(), LocationKey::default()),
            resources: StateResources::new(),
            board: Board::new(),
            location_counter: 100,
        }
    }

    pub async fn start_game(mut self: &mut Self, data: &str, communicator: &mut GameCommunicator) -> Result<()> {
        self.player_1.deck = self.resources.add_location(0, Box::new(CardCollection::new()));
        self.player_2.deck = self.resources.add_location(1, Box::new(CardCollection::new()));
        self.player_1.hand = self.resources.add_location(2, Box::new(CardCollection::new()));
        self.player_2.hand = self.resources.add_location(3, Box::new(CardCollection::new()));
        self.board.side_1.hero = self.resources.add_location(4, Box::new(CardSlot::new()));
        self.board.side_2.hero = self.resources.add_location(5, Box::new(CardSlot::new()));
        self.board.side_1.landscape = self.resources.add_location(6, Box::new(CardSlot::new()));
        self.board.side_2.landscape = self.resources.add_location(7, Box::new(CardSlot::new()));
        self.board.side_1.graveyard = self.resources.add_location(8, Box::new(CardCollection::new()));
        self.board.side_2.graveyard = self.resources.add_location(9, Box::new(CardCollection::new()));
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
        let card_key = get_tag("card", data)?.parse::<CardKey>()?;
        let target_location_key = get_tag("location", data)?.parse::<LocationKey>()?;

        let card = self.resources.card_instances.get(&card_key).context("Unable to find card")?;

        if card.location == target_location_key {
            return Ok(());
        }

        if card.owner != self.current_turn {
            communicator.send_error("Can't play card out of turn").await?;
            communicator.send_game_instruction( InstructionToClient::MoveCard { card: card.instance_id, to: card.location }).await?;
            return Ok(());
        }

        if card.location != self.get_player(card.owner).hand {
            communicator.send_error("Can't play card from this location").await?;
            communicator.send_game_instruction( InstructionToClient::MoveCard { card: card_key, to: card.location }).await?;
            return Ok(());
        }

        if self.board.get_side(card.owner).field.contains(&target_location_key) == false {
            communicator.send_error("Can't play card to this location").await?;
            communicator.send_game_instruction( InstructionToClient::MoveCard { card: card_key, to: card.location }).await?;
            return Ok(());
        }

        self.resources.move_card(card_key, target_location_key, communicator).await?;

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

    pub fn trigger_card_events(&mut self, resources: &mut StateResources, trigger_owner: PlayerId, communicator: &mut GameCommunicator, trigger: BehaviorTrigger, context: &TriggerContext) -> Result<()> {
        let mut locations = vec![
            self.board.side_1.hero, self.board.side_1.landscape, self.board.side_1.graveyard,
            self.board.side_1.hero, self.board.side_1.landscape, self.board.side_2.graveyard,
        ];

        locations.append(&mut self.board.side_1.field.clone());
        locations.append(&mut self.board.side_2.field.clone());

        for location in locations {
            let location = resources.locations.get(&location).unwrap();

            for key in location.get_cards() {
                let card = resources.card_instances.get(&key).unwrap();

                for behavior in &card.behaviors {
                    behavior.trigger(trigger, trigger_owner, context, self, communicator, key)?;
                }
            }
        }

        Ok(())
    }
}
