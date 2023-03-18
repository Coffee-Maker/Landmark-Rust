use std::collections::HashMap;
use std::fmt::{Display, Formatter};

use color_eyre::eyre;
use color_eyre::eyre::{anyhow, ContextCompat, eyre};
use color_eyre::owo_colors::OwoColorize;
use eyre::Result;
use slotmap::{DefaultKey, SlotMap};
use tokio::net::TcpStream;
use tokio_tungstenite::WebSocketStream;

use crate::game::board::{Board, BoardSide};
use crate::game::card_collection::CardCollection;
use crate::game::card_slot::CardSlot;
use crate::game::cards::card::{CardData, CardRegistry, CardCategory};
use crate::game::cards::card_behavior::BehaviorTrigger;
use crate::game::game_communicator::GameCommunicator;
use crate::game::game_state::PlayerId::{Player1, Player2};
use crate::game::instruction::{Instruction, InstructionQueue};
use crate::game::location::Location;
use crate::game::player::Player;
use crate::game::tag::get_tag;
use crate::game::trigger_context::TriggerContext;

pub async fn game_service(websocket: WebSocketStream<TcpStream>) -> Result<()> {
    println!("Starting game service");

    let mut communicator = GameCommunicator::new(websocket).await;

    let mut game_state = GameState::new();
    loop {
        let msg = communicator.read_message().await?;

        println!("Received message: {}", msg);
        if msg.len() < 3 {
            communicator.send_error("Given information was too short to be meaningful.").await?;
            continue;
        }

        let message = msg.into_text().unwrap();

        let instruction = &message[0..3];
        let data = &message[3..];

        let mut res = match instruction {
            "stg" => game_state.start_game(data, &mut communicator).await,          // Start the game
            "mve" => game_state.player_move_card(data, &mut communicator).await,    // Move a card
            "sel" => todo!(),                                                       // Select a card
            "eff" => todo!(),                                                       // Use a card effect // TODO COFFEE DO A HIGHLIGHT COMMAND // Yuh, good idea
            "trg" => todo!(),                                                       // Target a unit
            "atk" => todo!(),                                                       // Attack a unit
            "opt" => todo!(),                                                       // Select a presented option
            "eng" => Ok(game_state = GameState::new()),                             // End the game
            _ => Err(anyhow!("Unknown instruction: {}", instruction)),
        };

        if res.is_ok() {
            res = communicator.process_instructions(&mut game_state).await;
        }
        
        match res {
            Ok(_) => {}
            Err(e) => {
                communicator.send_error(&e.to_string()).await?;
                println!("Error: {}", e.to_string().red());
            }
        }
    }
}

pub type ObjKey = DefaultKey;
pub type LocationKey = ObjKey;
pub type CardKey = ObjKey;
pub type ServerInstanceId = u64;

type ThreadSafeLocation = dyn Location + Send + Sync;

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
    pub locations: SlotMap<LocationKey, Box<ThreadSafeLocation>>,
    pub location_keys: HashMap<ServerInstanceId, LocationKey>,
    pub card_instances: SlotMap<CardKey, CardData>,
    pub card_keys: HashMap<ServerInstanceId, CardKey>,
    pub card_registry: CardRegistry,
    pub board: Board,

    location_counter: ServerInstanceId,
}

impl GameState {
    pub fn new() -> Self {
        Self {
            current_turn: Player1,
            player_1: Player::new(Player1, LocationKey::default(), LocationKey::default()),
            player_2: Player::new(Player2, LocationKey::default(), LocationKey::default()),
            locations: SlotMap::new(),
            location_keys: HashMap::new(),
            card_instances: SlotMap::<CardKey, CardData>::new(),
            card_keys: HashMap::new(),
            card_registry: CardRegistry::from_directory("data/cards").unwrap(),
            board: Board::new(),
            location_counter: 100,
        }
    }

    pub fn reset_game(&mut self, communicator: &mut GameCommunicator) {
        for (key, _location) in &self.locations {
            Self::clear_location(&mut communicator.queue, key)
        }
    }

    pub fn clear_location(queue: &mut InstructionQueue, location: LocationKey) {
        queue.enqueue(Instruction::Clear { location })
    }

    pub async fn start_game(mut self: &mut Self, data: &str, communicator: &mut GameCommunicator) -> Result<()> {
        communicator.send_game_instruction(self, &Instruction::StartGame {}).await?;

        self.player_1.deck = self.add_location(0, Box::new(CardCollection::new()));
        self.player_2.deck = self.add_location(1, Box::new(CardCollection::new()));
        self.player_1.hand = self.add_location(2, Box::new(CardCollection::new()));
        self.player_2.hand = self.add_location(3, Box::new(CardCollection::new()));
        self.board.side_1.hero = self.add_location(4, Box::new(CardSlot::new()));
        self.board.side_2.hero = self.add_location(5, Box::new(CardSlot::new()));
        self.board.side_1.landscape = self.add_location(6, Box::new(CardSlot::new()));
        self.board.side_2.landscape = self.add_location(7, Box::new(CardSlot::new()));
        self.board.side_1.graveyard = self.add_location(8, Box::new(CardCollection::new()));
        self.board.side_2.graveyard = self.add_location(9, Box::new(CardCollection::new()));
        self.reset_game(communicator);

        // Set thaum
        self.player_1.set_thaum(0, &mut communicator.queue);
        self.player_2.set_thaum(0, &mut communicator.queue);

        // Parse decks
        let deck1string = get_tag("deck1", data)?;
        let deck2string = get_tag("deck2", data)?;
        self.player_1.populate_deck(&deck1string[..], &mut communicator.queue)?;
        self.player_2.populate_deck(&deck2string[..], &mut communicator.queue)?;
        communicator.process_instructions(&mut self).await?;
        self.player_1.prepare_deck(&self, &mut communicator.queue)?;
        self.player_2.prepare_deck(&self, &mut communicator.queue)?;
        communicator.process_instructions(&mut self).await?;
        self.prepare_landscape(Player1, communicator).await?;
        self.prepare_landscape(Player2, communicator).await?;
        communicator.process_instructions(&mut self).await?;

        // Add landscape slots


        for _ in 0..5 {
            self.player_1.draw_card(self, communicator)?;
            self.player_2.draw_card(self, communicator)?;
        }

        // Set random turn
        Self::set_turn(if fastrand::bool() { Player1 } else { Player2 }, communicator);

        Ok(())
    }

    pub async fn player_move_card(&mut self, data: &str, communicator: &mut GameCommunicator) -> Result<()> {
        let card_id = get_tag("card", data)?.parse::<ServerInstanceId>()?;
        let card_key = self.card_keys.get(&card_id).context("Unable to find target card key")?.clone();
        let target_location = get_tag("location", data)?.parse::<ServerInstanceId>()?;
        let target_location_key = self.location_keys.get(&target_location).context("Unable to find target location")?;

        let card = self.card_instances.get(card_key).context("Unable to find card")?;

        if card.location == *target_location_key {
            return Ok(());
        }

        if card.owner != self.current_turn {
            communicator.send_error("Can't play card out of turn").await?;
            Self::move_card(&mut communicator.queue, card_key, card.location);
            return Ok(());
        }

        if card.location != self.get_player(card.owner).hand {
            communicator.send_error("Can't play card from this location").await?;
            Self::move_card(&mut communicator.queue, card_key, card.location);
            return Ok(());
        }

        if self.get_side(card.owner).field.contains(target_location_key) == false {
            communicator.send_error("Can't play card to this location").await?;
            Self::move_card(&mut communicator.queue, card_key, card.location);
            return Ok(());
        }

        Self::move_card(&mut communicator.queue, card_key, *target_location_key);

        match &card.card_category {
            CardCategory::Unit { .. } => {
                communicator.queue.enqueue(Instruction::NotifySummon { card: card_key })
            }
            _ => {
                return Ok(());
            }
        }

        Ok(())
    }

    pub fn set_turn(player: PlayerId, comm: &mut GameCommunicator) {
        comm.queue.enqueue(Instruction::SetTurn { player });
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

    pub fn get_side(&self, id: PlayerId) -> &BoardSide {
        match id {
            Player1 => &self.board.side_1,
            Player2 => &self.board.side_2,
        }
    }

    pub fn get_side_mut(&mut self, id: PlayerId) -> &mut BoardSide {
        match id {
            Player1 => &mut self.board.side_1,
            Player2 => &mut self.board.side_2,
        }
    }

    pub fn move_card(queue: &mut InstructionQueue, card: CardKey, to: LocationKey) {
        queue.enqueue(Instruction::MoveCard { card, to });
    }

    pub fn trigger_card_events(&self, trigger_owner: PlayerId, communicator: &mut GameCommunicator, trigger: BehaviorTrigger, context: &TriggerContext) -> Result<()> {
        let mut locations = Vec::new();
        locations.push(self.board.side_1.hero);
        locations.push(self.board.side_2.hero);
        locations.push(self.board.side_1.landscape);
        locations.push(self.board.side_2.landscape);
        locations.append(&mut self.board.side_1.field.clone()); // Todo: Ask if this is the best way to do this
        locations.append(&mut self.board.side_2.field.clone());
        locations.push(self.board.side_1.graveyard);
        locations.push(self.board.side_2.graveyard);

        for location in locations {
            let location = self.locations.get(location).unwrap();

            for key in location.get_cards() {
                let card = self.card_instances.get(key).unwrap();

                for behavior in card.behaviors.clone() { // Todo: <- clone is bad prolly
                    behavior.trigger(trigger, trigger_owner, context, self, communicator, key)?;
                }
            }
        }

        Ok(())
    }

    pub fn add_location(&mut self, lid: ServerInstanceId, mut location: Box<ThreadSafeLocation>) -> LocationKey {
        location.set_lid(lid);

        let key = self.locations.insert(location);
        self.location_keys.insert(lid, key);

        key
    }
}
