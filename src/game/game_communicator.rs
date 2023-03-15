use std::net::TcpStream;
use std::ops::{Deref, DerefMut};
use tungstenite::{Message, WebSocket};
use color_eyre::Result;
use crate::game::instruction::Instruction;

pub struct GameCommunicator {
    websocket: WebSocket<TcpStream>,
}

impl GameCommunicator {
    pub fn new(websocket: WebSocket<TcpStream>) -> Self {
        Self { websocket }
    }

    pub fn send_info(&mut self, info: &str) -> Result<()> {
        self.websocket.write_message(Message::Text(format!("inf{}", info)))?;
        Ok(())
    }

    pub fn send_error(&mut self, error: &str) -> Result<()> {
        self.websocket.write_message(Message::Text(format!("err{}", error)))?;
        Ok(())
    }

    pub fn send_instruction(&mut self, instruction: &Instruction) -> Result<()> {
        self.websocket.write_message(Message::Text(format!("{}", instruction.build())))?;
        Ok(())
    }
}

impl Deref for GameCommunicator {
    type Target = WebSocket<TcpStream>;

    fn deref(&self) -> &Self::Target {
        &self.websocket
    }
}

impl DerefMut for GameCommunicator {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.websocket
    }
}