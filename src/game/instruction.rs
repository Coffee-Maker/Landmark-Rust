use async_recursion::async_recursion;
use color_eyre::Result;

use crate::game::animation_presets::AnimationPreset;
use crate::game::tokens::token_instance::TokenInstance;
use crate::game::game_communicator::GameCommunicator;
use crate::game::id_types::{TokenInstanceId, LocationId, PlayerId, PromptInstanceId, ServerInstanceId};
use crate::game::player::Player;
use crate::game::prompts::PromptType;
use crate::game::tag::Tag;

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
    MoveToken {
        token: TokenInstanceId,
        to: LocationId,
    },
    CreateToken {
        token_data: TokenInstance,
        instance_id: TokenInstanceId,
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
        token: TokenInstanceId,
    },
    AddPrompt {
        prompt_instance_id: PromptInstanceId,
        prompt_type: PromptType,
    },
    RemovePrompt {
        prompt_instance_id: PromptInstanceId,
    },
    UpdateData {
        token_data: TokenInstance,
    },
    UpdateBehaviors {
        token_data: TokenInstance,
    },
    AddEquipmentSlot {
        token: TokenInstanceId,
        slot_location_id: LocationId
    },
    Animate {
        token: TokenInstanceId,
        location: LocationId,
        duration: f32,
        preset: AnimationPreset,
    },
    Reveal {
        token: TokenInstanceId,
    },
    EndGame {
        winner: PlayerId
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
                    "add_slot|{}{}{}{}",
                    Tag::U64(3).build()?,
                    Tag::Player(player_id).build()?,
                    Tag::U64(index).build()?,
                    Tag::LocationId(location_id).build()?,
                )
            }
            InstructionToClient::SetThaum { player_id, amount } => {
                format!(
                    "set_thaum|{}{}{}",
                    Tag::U64(2).build()?,
                    Tag::Player(player_id).build()?,
                    Tag::U64(amount as u64).build()?
                )
            }
            InstructionToClient::MoveToken { token, to } => {
                format!(
                    "move_token|{}{}{}",
                    Tag::U64(2).build()?,
                    Tag::TokenInstanceId(token).build()?,
                    Tag::LocationId(to).build()?
                )
            }
            InstructionToClient::CreateToken {
                token_data,
                instance_id,
                location_id,
                player_id,
            } => {
                format!(
                    "create_token|{}{}{}{}{}",
                    Tag::U64(4).build()?,
                    Tag::TokenInstanceData(token_data).build()?,
                    Tag::TokenInstanceId(instance_id).build()?,
                    Tag::Player(player_id).build()?,
                    Tag::LocationId(location_id).build()?,
                )
            }
            InstructionToClient::PassTurn { player_id } => {
                format!("set_turn|{}{}", Tag::U64(1).build()?, Tag::Player(player_id).build()?)
            }
            InstructionToClient::ClearLocation { location } => {
                format!("clear_location|{}{}", Tag::U64(1).build()?, Tag::LocationId(location).build()?)
            }
            InstructionToClient::AddPrompt {
                prompt_instance_id,
                prompt_type,
            } => {
                let bind_target = match prompt_type {
                    PromptType::SelectToken(token_id) => token_id.0.to_string(),
                    PromptType::AttackToken(token_id) => token_id.0.to_string(),
                    PromptType::SelectFieldSlot(location_id) => location_id.0.to_string(),
                };
                format!("add_prompt|{}{}{}{:?}", Tag::U64(3).build()?, Tag::PromptInstanceId(prompt_instance_id).build()?, Tag::String(bind_target).build()?, Tag::String(prompt_type.to_string()).build()?)
            }
            InstructionToClient::RemovePrompt {
                prompt_instance_id,
            } => {
                format!("remove_prompt|{}{}", Tag::U64(1).build()?, Tag::PromptInstanceId(prompt_instance_id).build()?)
            }
            InstructionToClient::UpdateData { token_data } => {
                format!("update_data|{}{}{}", Tag::U64(2).build()?, Tag::TokenInstanceId(token_data.instance_id).build()?, Tag::TokenInstanceData(token_data).build()?)
            }
            InstructionToClient::UpdateBehaviors { token_data } => {
                format!("update_behaviors|{}{}{}", Tag::U64(2).build()?, Tag::TokenInstanceId(token_data.instance_id).build()?, Tag::TokenBehaviors(token_data).build()?)
            }
            InstructionToClient::AddEquipmentSlot { token, slot_location_id } => {
                format!("add_equipment_slot|{}{}{}", Tag::U64(2).build()?, Tag::TokenInstanceId(token).build()?, Tag::LocationId(slot_location_id).build()?)
            }
            InstructionToClient::Animate { token, location, duration, preset } => {
                format!("animate|{}{}{}{}{}", Tag::U64(4).build()?, Tag::TokenInstanceId(token).build()?, Tag::LocationId(location).build()?, Tag::F32(duration).build()?, Tag::String(preset.to_string()).build()?)
            }
            InstructionToClient::Reveal { token } => {
                format!("reveal|{}{}", Tag::U64(1).build()?, Tag::TokenInstanceId(token).build()?)
            }
            InstructionToClient::EndGame { winner } => {
                format!("end_game|{}{}", Tag::U64(1).build()?, Tag::Player(winner).build()?)
            }
            _ => todo!("instruction not implemented"),
        })
    }
}
