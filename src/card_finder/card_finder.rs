use std::net::TcpStream;
use tungstenite::WebSocket;

use color_eyre::Result;
use crate::game::cards::card::{CardRegistry, CardType};
use crate::game::game_communicator::GameCommunicator;
use crate::game::game_state::PlayerID::Player1;

pub fn finder_service(websocket: WebSocket<TcpStream>) -> Result<()> {
    println!("Starting card finder service");

    let mut comm = GameCommunicator::new(websocket);
    let registry = CardRegistry::from_directory("data/cards")?;
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
            "crd" => {
                let card = registry.create_card(data, 0, Player1)?;

                let id = card.card_id.clone();
                let name = card.name.clone();
                let description = card.description.clone();
                let cost = card.cost;
                let mut health = 0;
                let mut attack = 0;
                let mut defense = 0;
                let types = card.card_types.join(", ");
                let card_type = match card.card_type {
                    CardType::Hero => 0,
                    CardType::Landscape { slots: _slots } => 1,
                    CardType::Unit { attack: a, health: h, defense: d } => {
                        attack = a;
                        health = h;
                        defense = d;
                        2
                    }
                    CardType::Item => 3,
                    CardType::Command => 4,
                };

                let msg = format!("crd{id};;{card_type};;{name};;{description};;{cost};;{health};;{defense};;{attack};;{types};;");
                
                comm.send_raw(&msg)?;
            }
            _ => {}
        }
    }
}