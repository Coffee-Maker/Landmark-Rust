use std::collections::{vec_deque, VecDeque};

use color_eyre::Result;
use futures_util::{SinkExt, StreamExt};
use owo_colors::{OwoColorize, Rgb, Style};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

use crate::game::cards::card_deserialization::{CardBehavior, CardBehaviorTriggerWhenName};
use crate::game::instruction::InstructionToClient;

pub struct GameCommunicator {
    websocket: WebSocketStream<TcpStream>,
    server_style: Style,
    client_style: Style,
}

impl GameCommunicator {
    pub fn new(websocket: WebSocketStream<TcpStream>) -> Self {
        Self {
            websocket,
            server_style: Style::new().color(Rgb(50, 150, 200)).bold(),
            client_style: Style::new().color(Rgb(50, 200, 150)).bold(),
        }
    }

    pub async fn send_info(&mut self, info: &str) -> Result<()> {
        println!("{} {}: {}", "(Server)".style(self.server_style), "Info".color(Rgb(150, 150, 150)), info);
        self.websocket.send(Message::Text(format!("info|{}", info))).await?;
        Ok(())
    }

    pub async fn send_warning(&mut self, warning: &str) -> Result<()> {
        println!("{} {}: {}", "(Server)".style(self.server_style), "Warning".color(Rgb(250, 200, 30)), warning);
        self.websocket.send(Message::Text(format!("warn|{}", warning))).await?;
        Ok(())
    }

    pub async fn send_error(&mut self, error: &str) -> Result<()> {
        println!("{} {}: {}", "(Server)".style(self.server_style), "Error".color(Rgb(255, 50, 50)), error);
        self.websocket.send(Message::Text(format!("error|{}", error))).await?;
        Ok(())
    }

    pub async fn send_game_instruction(
        &mut self,
        instruction: InstructionToClient,
    ) -> Result<()> {
        let message = instruction.clone().build().await?;
        println!("{} {}: {}", "(Server)".style(self.server_style), "Command".color(Rgb(80, 150, 120)), message.color(Rgb(120, 120, 120)));
        self.websocket.send(Message::Text(message)).await?;
        Ok(())
    }

    pub async fn send_raw(&mut self, msg: &str) -> Result<()> {
        self.websocket.send(Message::Text(msg.into())).await?;
        Ok(())
    }

    pub async fn read_message(&mut self) -> Result<Message> {
        let msg = self.websocket.next().await.expect("Failed to read message")?;
        println!("{} {}", "(Client)".style(self.client_style), msg);
        Ok(msg)
    }
}