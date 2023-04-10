use std::char::MAX;
use std::collections::{HashMap, VecDeque};
use std::fmt::{Display, Formatter};
use std::num::ParseIntError;
use std::str::FromStr;

use color_eyre::eyre;
use color_eyre::eyre::{Context, ContextCompat, eyre};
use eyre::Result;
use futures_util::FutureExt;
use once_cell::sync::Lazy;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::WebSocketStream;

use crate::game::animation_presets::AnimationPreset;
use crate::game::board::Board;
use crate::game::game_communicator::GameCommunicator;
use crate::game::game_context::{ContextValue, GameContext};
use crate::game::id_types::{location_ids, LocationId, PlayerId, PromptInstanceId, ServerInstanceId, TokenInstanceId};
use crate::game::id_types::PlayerId::{Player1, Player2};
use crate::game::instruction::InstructionToClient;
use crate::game::new_state_machine::StateMachine;
use crate::game::player::Player;
use crate::game::prompts::{PromptCallback, PromptCallbackClosure, PromptCallbackResult, PromptInstance, PromptProfile, PromptType};
use crate::game::state_resources::StateResources;
use crate::game::tag::get_tag;
use crate::game::tokens::token_behaviors;
use crate::game::tokens::token_behaviors::TokenBehaviorResult;
use crate::game::tokens::token_deserializer::{TokenBehaviorTriggerWhenName, TokenCategory};
use crate::game::tokens::token_instance::TokenInstance;
use crate::game::tokens::token_registry::TokenRegistry;

pub async fn game_service(websocket: WebSocketStream<TcpStream>) -> Result<()> {
    println!("Starting game service");

    let mut communicator = GameCommunicator::new(websocket);
    let mut state = StateMachine::new();
    let mut resources = StateResources::new();
    let mut current_callback: Option<PromptCallback> = None;
    let mut callback_context = GameContext::new();

    loop {
        let msg = communicator.read_message().await?;

        let message = msg.into_text().unwrap();

        let [instruction, data] = message.split('|').collect::<Vec<_>>()[..] else {
            println!("Could not execute invalid instruction.");
            continue;
        };

        if let Some(callback) = &mut current_callback {
            if instruction == "callback" {
                match callback.execute(data.to_string(), &mut callback_context, &mut state, &mut resources, &mut communicator)? {
                    PromptCallbackResult::Keep => { },
                    PromptCallbackResult::End(new_callback) => {
                        callback.cancel(&mut communicator).await?;
                        if let Some(callback) = &new_callback {
                            callback.create_instructions(&mut communicator).await?;
                        }
                        current_callback = new_callback;

                        if current_callback.is_some() { continue }
                    }
                }
            } else {
                if callback.cancelable {
                    callback.cancel(&mut communicator).await?;
                } else {
                    todo!() // How to properly refuse a player action?
                }
            }
        }

        let result = match instruction {
            "start_game" => {
                state = StateMachine::new();
                state.start_game(data, &mut resources, &mut communicator).await
            },
            "move_token" => {
                let token_instance_id = get_tag("token", data)?.parse::<TokenInstanceId>()?;
                let target_location_id = get_tag("location", data)?.parse::<LocationId>()?;
                match resources.token_instances.get(&token_instance_id).context("Token to move not found")?.token_data.token_category {
                    TokenCategory::Unit {..} => {
                        if resources.can_player_summon_unit(token_instance_id, target_location_id, &mut communicator).await? {
                            state.summon_token(token_instance_id, target_location_id);
                        }
                    },
                    TokenCategory::Item {..} => {
                        if resources.can_player_equip_item(token_instance_id, target_location_id, &mut communicator).await? {
                            let equipping_unit_id = resources.equipment_slot_owners.get(&target_location_id).context("This location is not an equipment slot")?;
                            state.equip_item(*equipping_unit_id, token_instance_id);
                        }
                    },
                    _ => {}
                }
                Ok(())
            }
            "pass_turn" => {
                let mut cancel = false;
                if let Some(callback) = &mut current_callback {
                    if callback.cancelable {
                        callback.cancel(&mut communicator).await?;
                        current_callback = None;
                    } else {
                        cancel = true;
                    }
                }
                if cancel == false {
                    resources.set_current_turn(resources.current_turn.opponent(), &mut state, &mut communicator).await?;
                }
                Ok(())
            },
            "callback" => { Ok(()) }
            _ => Err(eyre!("Unknown instruction: {}", instruction)),
        };

        match result {
            Ok(_) => {

            }
            Err(e) => {
                communicator.send_error(&e.to_string()).await?;
            }
        }

        if let Some(callback) = state.process(&mut resources, &mut communicator).await? {
            callback.create_instructions(&mut communicator).await?;
            current_callback = Some(callback);
        } else {
            let callback = resources.show_selectable_tokens(&mut communicator).await?;
            callback.create_instructions(&mut communicator).await?;
            current_callback = Some(callback);
        }
    }
}