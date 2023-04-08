#![feature(let_chains)]
#![feature(int_roundings)]
#![allow(unused)]

use std::fs;
use color_eyre::eyre::{eyre, Result};
use once_cell::sync::Lazy;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::handshake::server::Request;
use tokio_tungstenite::tungstenite::handshake::server::Response;

use game::game_state;
use crate::game::tokens::token_deserializer::{TokenData, TokenBehaviorTriggerWhenActivator};
use crate::game::tokens::token_registry::TokenRegistry;

mod game;
mod token_finder;

pub static TOKEN_REGISTRY: Lazy<Mutex<TokenRegistry>> = Lazy::new(|| {
    Mutex::new(TokenRegistry::from_directory("data/tokens").unwrap())
});

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    game::tokens::token_registry::TokenRegistry::from_directory("data/tokens")?;

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
            "/tokenfinder" => {
                service_type = ServiceType::TokenFinder;
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
                eprintln!("{:?}", e);
            }
        },
        ServiceType::TokenFinder => {
            token_finder::token_finder::finder_service(websocket).await;
        },
    }
}

// An enum for service type
enum ServiceType {
    None,
    Game,
    TokenFinder,
}
