use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::net::TcpStream;
use std::rc::Rc;

use color_eyre::eyre;
use eyre::Result;
pub use tungstenite::WebSocket;
use crate::game::cards::card::{CardData, CardInstance, CardRegistry};
use crate::game::game_communicator::GameCommunicator;
use crate::game::instruction::Instruction;
use crate::game::location::{Location, LocationInstance};

use crate::game::player::{CardCollection, Player};
use crate::game::tag::get_tag;

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
        
        match instruction { 
            "stg" => game_state.start_game(data, &mut comm)?, // Start the game
            "plc" => todo!(), // Play a card
            "eff" => todo!(), // Use a card effect
            "sel" => todo!(), // Select a card
            "trg" => todo!(), // Target a unit
            "atk" => todo!(), // Attack a unit
            "opt" => todo!(), // Select a presented option
            "eng" => todo!(), // End the game
            _ => {}
        }

        //game_state.player1.set_thaum(2, &mut comm)?;
    }
}

pub struct GameState<'a> {
    pub player1: Option<Player<'a>>,
    pub player2: Option<Player<'a>>,
    pub locations: HashMap<u32, Box<dyn Location>>,
    pub card_instances: HashMap<u32, CardData<'a>>,
    pub card_registry: CardRegistry,
    pub queue: VecDeque<Instruction<'a>>,
}

impl<'a> GameState<'a> {
    pub fn new() -> Self {
        
        let mut locations = HashMap::new();

        Self {
            player1: None,
            player2: None,
            locations,
            card_instances: HashMap::new(),
            card_registry: CardRegistry::new(),
            queue: VecDeque::new(),
        }
    }
    
    pub fn start_game(&mut self, data: &str, comm: &mut GameCommunicator) -> Result<()> {
        self.add_location(0, Box::new(CardCollection::new()));
        self.add_location(1, Box::new(CardCollection::new()));
        self.add_location(2, Box::new(CardCollection::new()));
        self.add_location(3, Box::new(CardCollection::new()));
        let deck1 = self.locations.get(&0).unwrap();
        let deck2 = self.locations.get(&1).unwrap();
        let hand1 = self.locations.get(&2).unwrap();
        let hand2 = self.locations.get(&3).unwrap();

        self.player1 = Some(Player::new(0, deck1, hand1));
        self.player2 = Some(Player::new(0, deck2, hand2));
        
        // Parse decks
        let deck1string = get_tag("deck1", data)?;
        let deck2string = get_tag("deck2", data)?;
        self.player1.as_ref().unwrap().populate_deck(&deck1string[..], &self.card_registry, &mut self.queue)?;
        //self.player2.as_ref().unwrap().populate_deck(&deck2string[..], &self.card_registry, &mut self.queue)?;
        
        //self.draw_cards();
        
        Ok(())
    }
    
    pub fn draw_cards(&mut self) {
        todo!()
    }

    pub fn add_location(&mut self, lid: u32, mut location: Box<dyn Location>) -> LocationInstance {
        location.set_lid(lid);
        self.locations.insert(lid, location);
        self.locations.get(&lid).unwrap()
    }
}