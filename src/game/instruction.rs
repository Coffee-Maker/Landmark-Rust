use crate::game::game_communicator::GameCommunicator;
use crate::game::tag::Tag;
use async_recursion::async_recursion;

use crate::game::cards::card_instance::CardInstance;
use crate::game::game_state::GameState;
use color_eyre::Result;

use crate::game::prompts::PromptType;
use crate::game::id_types::{CardInstanceId, LocationId, PlayerId, PromptInstanceId, ServerInstanceId};

pub enum InstructionPostProcess {
    Ok,
    WaitForCallback {
        callback: Box<dyn FnOnce(&str, &mut GameState, &mut GameCommunicator) -> Result<bool>>,
        cancellable: InstructionPostProcessCancelType,
    },
}

pub enum InstructionPostProcessCancelType {
    NotCancellable,
    Cancellable {
        on_cancel: Box<dyn FnOnce(&str, &mut GameState, &mut GameCommunicator) -> Result<()>>,
    },
}

#[derive(Clone)]
pub enum InstructionToClient {
    AddLandscapeSlot {
        player_id: PlayerId,
        index: u64,
        location_id: LocationId,
    },
    SetThaum {
        player_id: PlayerId,
        amount: u32,
    },
    MoveCard {
        card: CardInstanceId,
        to: LocationId,
    },
    DrawCard {
        player: PlayerId,
    },
    CreateCard {
        card_data: CardInstance,
        instance_id: CardInstanceId,
        location_id: LocationId,
        player_id: PlayerId,
    },
    PassTurn {
        player_id: PlayerId,
    },
    ClearLocation {
        location: LocationId,
    },
    Destroy {
        card: CardInstanceId,
    },
    AddPrompt {
        prompt_instance_id: PromptInstanceId,
        prompt_type: PromptType,
    },
    RemovePrompt {
        prompt_instance_id: PromptInstanceId,
    },
    UpdateData {
        card_data: CardInstance,
    }
}

impl InstructionToClient {
    #[async_recursion]
    pub async fn build(self) -> Result<String> {
        Ok(match self {
            InstructionToClient::AddLandscapeSlot {
                player_id,
                index,
                location_id,
            } => {
                format!(
                    "add_slot|{}{}{}",
                    Tag::Player(player_id).build()?,
                    Tag::U64(index).build()?,
                    Tag::LocationId(location_id).build()?,
                )
            }
            InstructionToClient::SetThaum { player_id, amount } => {
                format!(
                    "set_thaum|{}{}",
                    Tag::Player(player_id).build()?,
                    Tag::U64(amount as u64).build()?
                )
            }
            InstructionToClient::MoveCard { card, to } => {
                format!(
                    "move_card|{}{}",
                    Tag::CardInstanceId(card).build()?,
                    Tag::LocationId(to).build()?
                )
            }
            InstructionToClient::CreateCard {
                card_data,
                instance_id,
                location_id,
                player_id,
            } => {
                format!(
                    "create_card|{}{}{}{}",
                    Tag::CardData(card_data).build()?,
                    Tag::CardInstanceId(instance_id).build()?,
                    Tag::Player(player_id).build()?,
                    Tag::LocationId(location_id).build()?,
                )
            }
            InstructionToClient::PassTurn { player_id } => {
                format!("set_turn|{}", Tag::Player(player_id).build()?)
            }
            InstructionToClient::ClearLocation { location } => {
                format!("clear_location|{}", Tag::LocationId(location).build()?)
            }
            InstructionToClient::AddPrompt {
                prompt_instance_id,
                prompt_type,
            } => {
                let bind_target = match prompt_type {
                    PromptType::SelectCard(card_id) => card_id.0.to_string(),
                    PromptType::AttackCard(card_id) => card_id.0.to_string(),
                    PromptType::SelectFieldSlot(location_id) => location_id.0.to_string(),
                };
                format!("add_prompt|{}{}{:?}", Tag::PromptInstanceId(prompt_instance_id).build()?, Tag::String(bind_target).build()?, Tag::String(prompt_type.to_string()).build()?)
            }
            InstructionToClient::RemovePrompt {
                prompt_instance_id,
            } => {
                format!("remove_prompt|{}", Tag::PromptInstanceId(prompt_instance_id).build()?)
            }
            InstructionToClient::UpdateData { card_data } => {
                format!("update_data|{}{}", Tag::CardInstanceId(card_data.instance_id).build()?, Tag::CardData(card_data).build()?)
            }
            _ => todo!("instruction not implemented"),
        })
    }
}
