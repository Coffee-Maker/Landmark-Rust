#![feature(let_chains)]
#![allow(unused)]

use std::fs;
use color_eyre::eyre::{eyre, Result};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::handshake::server::Request;
use tokio_tungstenite::tungstenite::handshake::server::Response;

use game::game_state;
use crate::game::cards::card_deserialization::{Card, CardBehaviorTriggerWhenActivator};

mod game;
mod card_finder;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    game::cards::card_registry::CardRegistry::from_directory("data/cards")?;

    println!("Starting TcpListener");

    let server = TcpListener::bind("127.0.0.1:15076").await?;

    while let Ok((stream, _)) = server.accept().await {
        tokio::spawn(accept_connection(stream));
    }

    Ok(())
}

async fn accept_connection(stream: TcpStream) {
    let mut service_type = ServiceType::None;

    let callback = |req: &Request, response: Response| {
        // switch on the path
        match req.uri().path() {
            "/game" => {
                service_type = ServiceType::Game;
                Ok(response)
            }
            "/cardfinder" => {
                service_type = ServiceType::CardFinder;
                Ok(response)
            }
            _ => {
                service_type = ServiceType::None;
                Ok(response)
            }
        }
    };

    let websocket = tokio_tungstenite::accept_hdr_async(stream, callback).await.unwrap();

    match service_type {
        ServiceType::None => {
            println!("No service type found");
        }
        ServiceType::Game => {
            if let Err(e) = game_state::game_service(websocket).await {
                eprintln!("{}", e);
            }
        },
        ServiceType::CardFinder => {
            card_finder::card_finder::finder_service(websocket).await;
        },
    }
}

// An enum for service type
enum ServiceType {
    None,
    Game,
    CardFinder,
}
