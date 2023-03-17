use std::net::TcpListener;
use std::thread::spawn;

use color_eyre::eyre::Result;
use tungstenite::accept_hdr;
use tungstenite::handshake::server::Request;
use tungstenite::handshake::server::Response;

use game::game_state;

mod game;
mod card_finder;

/// A WebSocket echo server
fn main() -> Result<()> {
    color_eyre::install()?;

    println!("Starting TcpListener");

    let server = TcpListener::bind("127.0.0.1:15076").unwrap();

    for stream in server.incoming() {
        spawn(move || {
            //println!("Found incoming connection");
            let stream = stream.unwrap();

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

            let websocket = accept_hdr(stream, callback).unwrap();
            match service_type {
                ServiceType::None => {
                    println!("No service type found");
                    Ok(())
                }
                ServiceType::Game => game_state::game_service(websocket),
                ServiceType::CardFinder => card_finder::card_finder::finder_service(websocket),
            }.unwrap()
        });
    }

    Ok(())
}

// An enum for service type
enum ServiceType {
    None,
    Game,
    CardFinder,
}
