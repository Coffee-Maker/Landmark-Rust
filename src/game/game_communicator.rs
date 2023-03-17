use std::collections::VecDeque;
use crate::game::game_state::GameState;
use crate::game::instruction::Instruction;
use color_eyre::Result;
use std::net::TcpStream;
use std::ops::{Deref, DerefMut};
use tungstenite::{Message, WebSocket};

pub struct GameCommunicator {
    websocket: WebSocket<TcpStream>,
    pub queue: VecDeque<Instruction>,
}

impl GameCommunicator {
    pub fn new(websocket: WebSocket<TcpStream>) -> Self {
        Self {
            websocket,
            queue: VecDeque::new(),
        }
    }

    pub fn send_info(&mut self, info: &str) -> Result<()> {
        self.websocket
            .write_message(Message::Text(format!("inf{}", info)))?;
        Ok(())
    }

    pub fn send_error(&mut self, error: &str) -> Result<()> {
        self.websocket
            .write_message(Message::Text(format!("err{}", error)))?;
        Ok(())
    }

    pub fn send_game_instruction(
        &mut self,
        state: &mut GameState,
        instruction: &Instruction,
    ) -> Result<()> {
        let message = instruction.clone().build(state)?;
        println!("\x1b[32m{}\x1b[0m", message);
        self.websocket
            .write_message(Message::Text(message))?;
        Ok(())
    }

    pub fn send_raw(&mut self, msg: &str) -> Result<()> {
        self.websocket
            .write_message(Message::Text(msg.into()))?;
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
