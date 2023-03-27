use color_eyre::Result;
use tokio::net::TcpStream;
use tokio_tungstenite::WebSocketStream;
use crate::CARD_REGISTRY;
use crate::game::cards::token_deserializer::{TokenData, TokenCategory};
use crate::game::game_communicator::GameCommunicator;
use crate::game::tag::{get_tag, Tag};

pub async fn finder_service(websocket: WebSocketStream<TcpStream>) -> Result<()> {
    let mut communicator = GameCommunicator::new(websocket);
    loop {
        let msg = communicator.read_message().await?;

        let message = msg.into_text().unwrap();

        let [instruction, data] = message.split('|').collect::<Vec<_>>()[..] else {
            println!("Could not execute invalid instruction.");
            continue;
        };

        match instruction {
            "search" => {
                communicator.send_raw(&"clear_results|//0/!").await?;
                let registry = CARD_REGISTRY.lock().await;
                let mut message_to_send = String::new();
                for card in registry.card_registry.values().collect::<Vec<&&TokenData>>() {
                    message_to_send = format!("{}add_result|{}{}//INS//", message_to_send, Tag::U64(1).build()?, Tag::CardData(card.clone().clone()).build()?);
                    // Todo: Add behaviors in message
                }
                communicator.send_raw(&message_to_send).await?;
            },
            "get_deck_card" => {
                let registry = CARD_REGISTRY.lock().await;
                let card = registry.get_data(&get_tag(&"id", &data)?)?;
                communicator.send_raw(&format!("add_deck_card|{}{}{}", Tag::U64(2).build()?, Tag::String(get_tag(&"slot", &data)?).build()?, Tag::CardData(card.clone()).build()?)).await?;
            }
            _ => {}
        }
    }
}