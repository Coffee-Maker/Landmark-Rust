use crate::game::game_communicator::GameCommunicator;
use crate::game::tag::Tag;
use async_recursion::async_recursion;

use crate::game::game_state::{GameState, PlayerId, ServerInstanceId};
use color_eyre::Result;
use crate::game::cards::card::CardData;

use crate::game::highlight_type::HighlightType;

pub enum InstructionPostProcess {
    Ok,
    WaitForCallback {
        callback: Box<dyn FnOnce(&str, &mut GameState, &mut GameCommunicator) -> Result<bool>>,
        cancellable: InstructionPostProcessCancelType
    }
}

pub enum InstructionPostProcessCancelType {
    NotCancellable,
    Cancellable {
        on_cancel: Box<dyn FnOnce(&str, &mut GameState, &mut GameCommunicator) -> Result<()>>
    }
}

#[derive(Clone)]
pub enum InstructionToClient {
    AddLandscapeSlot {
        player_id: PlayerId,
        index: u64,
        location_id: ServerInstanceId,
    },
    SetThaum {
        player_id: PlayerId,
        amount: u32,
    },
    MoveCard {
        card: ServerInstanceId,
        to: ServerInstanceId,
    },
    DrawCard { player: PlayerId },
    CreateCard {
        card_data: CardData,
        instance_id: ServerInstanceId,
        location_id: ServerInstanceId,
        player_id: PlayerId,
    },
    PassTurn { player_id: PlayerId },
    ClearLocation { location: ServerInstanceId },
    Destroy { card: ServerInstanceId },
    HighlightCard { card: ServerInstanceId, highlight_type: HighlightType }
}

impl InstructionToClient {
    #[async_recursion]
    pub async fn build(self) -> Result<String> {
        Ok(match self {
            InstructionToClient::AddLandscapeSlot {
                player_id, index, location_id: lid
            } => format!(
                "add_slot|{}{}{}",
                Tag::Player(player_id).build()?,
                Tag::U64(index).build()?,
                Tag::ServerInstanceId(lid).build()?,
            ),
            InstructionToClient::SetThaum {
                player_id,
                amount,
            } => format!(
                "set_thaum|{}{}",
                Tag::Player(player_id).build()?,
                Tag::U64(amount as u64).build()?
            ),
            InstructionToClient::MoveCard {
                card, to
            } => format!(
                "move_card|{}{}",
                Tag::ServerInstanceId(card).build()?,
                Tag::ServerInstanceId(to).build()?
            ),
            InstructionToClient::CreateCard {
                card_data,
                instance_id,
                location_id,
                player_id,
            } => {
                format!(
                    "create_card|{}{}{}{}",
                    Tag::CardData(card_data).build()?,
                    Tag::ServerInstanceId(instance_id).build()?,
                    Tag::Player(player_id).build()?,
                    Tag::ServerInstanceId(location_id).build()?,
                )
            }
            InstructionToClient::PassTurn {
                player_id
            } => format!(
                "set_turn|{}",
                Tag::Player(player_id).build()?
            ),
            InstructionToClient::ClearLocation {
                location
            } => format!(
                "clear_location|{}",
                Tag::ServerInstanceId(location).build()?
            ),
            _ => todo!("instruction not implemented"),
        })
    }
}
