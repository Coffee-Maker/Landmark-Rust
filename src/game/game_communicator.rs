use color_eyre::Result;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;

use crate::game::game_state::GameState;
use crate::game::instruction::{Instruction, InstructionQueue};

pub struct GameCommunicator {
    websocket: WebSocketStream<TcpStream>,
    pub queue: InstructionQueue,
}

impl GameCommunicator {
    pub async fn new(websocket: WebSocketStream<TcpStream>) -> Self {
        Self {
            websocket,
            queue: InstructionQueue::new(),
        }
    }

    pub async fn process_instructions(&mut self, state: &mut GameState) -> Result<()> {
        while self.queue.len() > 0 {
            let ins = self.queue.dequeue().unwrap();
            ins.process(state, self).await?
        }
        Ok(())
    }

    pub async fn send_info(&mut self, info: &str) -> Result<()> {
        self.websocket.send(Message::Text(format!("inf{}", info))).await?;
        Ok(())
    }

    pub async fn send_error(&mut self, error: &str) -> Result<()> {
        println!("(Server) \x1b[31m\x1bError: {error}\x1b[0m");
        self.websocket.send(Message::Text(format!("err{}", error))).await?;
        Ok(())
    }

    pub async fn send_game_instruction(
        &mut self,
        state: &mut GameState,
        instruction: &Instruction,
    ) -> Result<()> {
        let message = instruction.clone().build(state)?;
        println!("(Server) \x1b[32m{}\x1b[0m", message);
        self.websocket.send(Message::Text(message)).await?;
        Ok(())
    }

    pub async fn send_raw(&mut self, msg: &str) -> Result<()> {
        self.websocket.send(Message::Text(msg.into())).await?;
        Ok(())
    }

    pub async fn read_message(&mut self) -> Result<Message> {
        self.websocket.next().await.expect("Failed to read message").map_err(|e| e.into())
    }
}