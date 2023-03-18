#![feature(let_chains)]

use color_eyre::eyre::Result;
use owo_colors::OwoColorize;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::handshake::server::Request;
use tokio_tungstenite::tungstenite::handshake::server::Response;

use game::game_state;

mod game;
mod card_finder;

#[tokio::main]
async fn main() -> Result<()> {
    println!("{} {}", "(Server)".cyan().bold(), "Info: Did you know that the dot on top of the letter 'i' is called a tittle?".bright_black());
    println!("{} {}", "(Server)".cyan().bold(), "Warning: I have no idea who uses these".yellow());
    println!("{} {}", "(Server)".cyan().bold(), "Error: You made something and then it didn't work :(".red());
    println!("{} {}", "(Server)".cyan().bold(), "Command: tst//do you/!//like my/!//command format/!".green());
    println!("{} {}", "(Client)".yellow().bold(), "Incoming message here");

    color_eyre::install()?;

    println!("Starting TcpListener");

    let server = TcpListener::bind("127.0.0.1:15076").await?;

    while let Ok((stream, _)) = server.accept().await {
        tokio::spawn(accept_connection(stream));
    }

    Ok(())
}

async fn accept_connection(stream: TcpStream) -> Result<()> {
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

    let websocket = tokio_tungstenite::accept_hdr_async(stream, callback).await?;

    match service_type {
        ServiceType::None => {
            println!("No service type found");
            Ok(())
        }
        ServiceType::Game => game_state::game_service(websocket).await,
        ServiceType::CardFinder => card_finder::card_finder::finder_service(websocket).await,
    }
}

// An enum for service type
enum ServiceType {
    None,
    Game,
    CardFinder,
}
