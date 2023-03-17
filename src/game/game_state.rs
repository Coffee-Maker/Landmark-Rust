use std::collections::{HashMap, VecDeque};
use std::fmt::{Display, Formatter};
use std::net::TcpStream;

use color_eyre::eyre;
use color_eyre::eyre::{anyhow, ContextCompat};
use color_eyre::owo_colors::OwoColorize;
use eyre::Result;
use slotmap::{DefaultKey, SlotMap};
use toml::{Table, Value};
pub use tungstenite::WebSocket;
use crate::game::board::{Board, BoardSide};
use crate::game::card_slot::CardSlot;

use crate::game::cards::card::{CardData, CardRegistry, CardType};
use crate::game::cards::card_behaviour::BehaviourTrigger;
use crate::game::game_communicator::GameCommunicator;
use crate::game::game_state::PlayerID::{Player1, Player2};
use crate::game::instruction::{Instruction, InstructionQueue};
use crate::game::location::Location;
use crate::game::player::{CardCollection, Player};
use crate::game::tag::get_tag;
use crate::game::trigger_context::TriggerContext;

pub fn game_service(websocket: WebSocket<TcpStream>) -> Result<()> {
    println!("Starting game service");

    let mut comm = GameCommunicator::new(websocket);

    let mut game_state = GameState::new();
    loop {
        let msg = comm.read_message().unwrap();

        println!("Received message: {}", msg);
        if msg.len() < 3 {
            comm.send_error("Given information was too short to be meaningful.")?;
            continue;
        }

        let message = msg.into_text().unwrap();

        let instruction = &message[0..3];
        let data = &message[3..];

        let mut res = match instruction {
            "stg" => game_state.start_game(data, &mut comm),       // Start the game
            "mve" => game_state.player_move_card(data, &mut comm), // Move a card
            "sel" => todo!(),                                       // Select a card
            "eff" => todo!(),                                       // Use a card effect // TODO COFFEE DO A HIGHLIGHT COMMAND // Yuh, good idea
            "trg" => todo!(),                                       // Target a unit
            "atk" => todo!(),                                       // Attack a unit
            "opt" => todo!(),                                       // Select a presented option
            "eng" => Ok(game_state = GameState::new()),                 // End the game
            _ => Err(anyhow!("Unknown instruction: {}", instruction)),
        };

        if res.is_ok() {
            res = game_state.process_instructions(&mut comm);
        }
        
        match res {
            Ok(_) => {}
            Err(e) => {
                comm.send_error(&e.to_string())?;
                println!("Error: {}", e.to_string().red());
            }
        }
    }
}

pub type ObjKey = DefaultKey;
pub type LocationKey = ObjKey;
pub type CardKey = ObjKey;
pub type ServerIID = u64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlayerID {
    Player1 = 0,
    Player2 = 1,
}

impl Display for PlayerID {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Player1 => write!(f, "Player 1"),
            Player2 => write!(f, "Player 2"),
        }
    }
}

pub struct GameState {
    pub current_turn: PlayerID,
    player1: Player,
    player2: Player,
    pub locations: SlotMap<LocationKey, Box<dyn Location>>,
    pub location_keys: HashMap<ServerIID, LocationKey>,
    pub card_instances: SlotMap<CardKey, CardData>,
    pub card_keys: HashMap<ServerIID, CardKey>,
    pub card_registry: CardRegistry,
    pub board: Board,

    location_counter: ServerIID,
}

impl GameState {
    pub fn new() -> Self {
        Self {
            current_turn: Player1,
            player1: Player::new(Player1, LocationKey::default(), LocationKey::default()),
            player2: Player::new(Player2, LocationKey::default(), LocationKey::default()),
            locations: SlotMap::<LocationKey, Box<dyn Location>>::new(),
            location_keys: HashMap::new(),
            card_instances: SlotMap::<CardKey, CardData>::new(),
            card_keys: HashMap::new(),
            card_registry: CardRegistry::from_directory("data/cards").unwrap(),
            board: Board::new(),
            location_counter: 100,
        }
    }

    pub fn process_instructions(&mut self, comm: &mut GameCommunicator) -> Result<()> {
        while comm.queue.len() > 0 {
            let ins = comm.queue.pop_front().unwrap();
            ins.process(self, comm)?
        }
        Ok(())
    }

    pub fn reset_game(&mut self, comm: &mut GameCommunicator) {
        for (key, _location) in &self.locations {
            Self::clear_location(&mut comm.queue, key)
        }
    }

    pub fn clear_location(queue: InstructionQueue, location: ObjKey) {
        queue.push_back(Instruction::Clear { location })
    }

    pub fn start_game(&mut self, data: &str, comm: &mut GameCommunicator) -> Result<()> {
        comm.send_game_instruction(self, &Instruction::StartGame {})?;

        self.player1.deck = self.add_location(0, Box::new(CardCollection::new()));
        self.player2.deck = self.add_location(1, Box::new(CardCollection::new()));
        self.player1.hand = self.add_location(2, Box::new(CardCollection::new()));
        self.player2.hand = self.add_location(3, Box::new(CardCollection::new()));
        self.board.side1.hero = self.add_location(4, Box::new(CardSlot::new()));
        self.board.side2.hero = self.add_location(5, Box::new(CardSlot::new()));
        self.board.side1.landscape = self.add_location(6, Box::new(CardSlot::new()));
        self.board.side2.landscape = self.add_location(7, Box::new(CardSlot::new()));
        self.board.side1.graveyard = self.add_location(8, Box::new(CardCollection::new()));
        self.board.side2.graveyard = self.add_location(9, Box::new(CardCollection::new()));
        self.reset_game(comm);

        // Set thaum
        self.player1.set_thaum(0, &mut comm.queue);
        self.player2.set_thaum(0, &mut comm.queue);

        // Parse decks
        let deck1string = get_tag("deck1", data)?;
        let deck2string = get_tag("deck2", data)?;
        self.player1.populate_deck(&deck1string[..], &mut comm.queue)?;
        self.player2.populate_deck(&deck2string[..], &mut comm.queue)?;
        self.process_instructions(comm)?;
        self.player1.prepare_deck(&self, &mut comm.queue)?;
        self.player2.prepare_deck(&self, &mut comm.queue)?;
        self.process_instructions(comm)?;
        self.prepare_landscape(Player1, comm)?;
        self.prepare_landscape(Player2, comm)?;
        self.process_instructions(comm)?;

        // Add landscape slots


        for _ in 0..5 {
            self.draw_card(Player1, comm)?;
            self.draw_card(Player2, comm)?;
        }

        // Set random turn
        Self::set_turn(if fastrand::bool() { Player1 } else { Player2 }, comm);

        Ok(())
    }

    fn prepare_landscape(&mut self, player: PlayerID, comm: &mut GameCommunicator) -> Result<()> {
        comm.send_info(&format!("Preparing landscape for {}", player))?;
        let card_key = self.locations.get(self.get_side(player).landscape).unwrap().get_card().context(format!("Landscape slot is missing card"))?;
        let landscape = self.card_instances.get(card_key).context("Landscape card is not a valid instance")?;
        let landscape = &landscape.card_type;
        match landscape {
            CardType::Landscape {
                slots,
            } => {
                let mut i = 0;
                for _slot in slots {
                    comm.queue.push_back(Instruction::AddLandscapeSlot {
                        player,
                        index: i,
                        lid: self.location_counter,
                    });
                    i += 1;
                    self.location_counter += 1;
                }
            }
            _ => Err(anyhow!("Landscape card is not a landscape"))?,
        }

        Ok(())
    }

    pub fn player_move_card(&mut self, data: &str, comm: &mut GameCommunicator) -> Result<()> {
        let card_id = get_tag("card", data)?.parse::<ServerIID>()?;
        let card_key = self.card_keys.get(&card_id).unwrap().clone();
        let target_location = get_tag("location", data)?.parse::<ServerIID>()?;
        let target_location_key = self.location_keys.get(&target_location).unwrap();

        let card = self.card_instances.get(card_key).unwrap();

        if card.location == *target_location_key {
            return Ok(());
        }

        if card.owner != self.current_turn {
            comm.send_error("Can't play card out of turn")?;
            Self::move_card(&mut comm.queue, card_key, card.location);
            return Ok(());
        }

        if card.location != self.get_player(card.owner).hand {
            comm.send_error("Can't play card from this location")?;
            Self::move_card(&mut comm.queue, card_key, card.location);
            return Ok(());
        }

        if self.get_side(card.owner).field.contains(target_location_key) == false {
            comm.send_error("Can't play card to this location")?;
            Self::move_card(&mut comm.queue, card_key, card.location);
            return Ok(());
        }

        Self::move_card(&mut comm.queue, card_key, *target_location_key);

        match &card.card_type {
            CardType::Unit { .. } => {
                comm.queue.push_back(Instruction::NotifySummon { card: card_key })
            }
            _ => {
                return Ok(());
            }
        }

        Ok(())
    }

    pub fn set_turn(player: PlayerID, comm: &mut GameCommunicator) {
        comm.queue.push_back(Instruction::SetTurn { player });
    }

    pub fn get_player(&self, id: PlayerID) -> &Player {
        match id {
            Player1 => &self.player1,
            Player2 => &self.player2,
        }
    }

    pub fn get_player_mut(&mut self, id: PlayerID) -> &mut Player {
        match id {
            Player1 => &mut self.player1,
            Player2 => &mut self.player2,
        }
    }

    pub fn get_side(&self, id: PlayerID) -> &BoardSide {
        match id {
            Player1 => &self.board.side1,
            Player2 => &self.board.side2,
        }
    }

    pub fn get_side_mut(&mut self, id: PlayerID) -> &mut BoardSide {
        match id {
            Player1 => &mut self.board.side1,
            Player2 => &mut self.board.side2,
        }
    }

    pub fn draw_card(&self, player_id: PlayerID, comm: &mut GameCommunicator) -> Result<()> {
        let player = self.get_player(player_id);
        let card = self.locations.get(player.deck).unwrap().get_card();
        match card {
            None => {
                // lose instantly
                todo!()
            }
            Some(c) => {
                comm.queue.push_back(Instruction::DrawCard { player: player_id });
                let card = self.card_instances.get(c).unwrap();
                let mut context = TriggerContext::new();
                context.add_card(self, c);
                let owned = card.owner == player_id;
                self.trigger_card_events(player_id, comm, BehaviourTrigger::DrawCard, &context)?;
            }
        }
        Ok(())
    }

    pub fn move_card(queue: InstructionQueue, card: CardKey, to: LocationKey) {
        queue.push_back(Instruction::MoveCard { card, to });
    }

    pub fn trigger_card_events(&self, trigger_owner: PlayerID, comm: &mut GameCommunicator, trigger: BehaviourTrigger, context: &TriggerContext) -> Result<()> {
        let mut locations = Vec::new();
        locations.push(self.board.side1.hero);
        locations.push(self.board.side2.hero);
        locations.push(self.board.side1.landscape);
        locations.push(self.board.side2.landscape);
        locations.append(&mut self.board.side1.field.clone()); // Todo: Ask if this is the best way to do this 
        locations.append(&mut self.board.side2.field.clone());
        locations.push(self.board.side1.graveyard);
        locations.push(self.board.side2.graveyard);
        for location in locations {
            let location = self.locations.get(location).unwrap();
            for key in location.get_cards() {
                let card = self.card_instances.get(key).unwrap();
                for behaviour in card.behaviours.clone() { // Todo: <- clone is bad prolly
                    behaviour.trigger(trigger, trigger_owner, context, self, comm, key)?;
                }
            }
        }

        Ok(())
    }

    pub fn add_location(&mut self, lid: ServerIID, mut location: Box<dyn Location>) -> LocationKey {
        location.set_lid(lid);
        let key = self.locations.insert(location);
        self.location_keys.insert(lid, key);
        key
    }
}
