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
use crate::game::card_collection::CardCollection;
use crate::game::card_slot::CardSlot;
use crate::game::cards::token_behaviors;
use crate::game::cards::token_behaviors::CardBehaviorResult;
use crate::game::cards::token_deserializer::{CardBehaviorTriggerWhenName, TokenCategory};
use crate::game::cards::card_instance::TokenInstance;
use crate::game::cards::card_registry::CardRegistry;
use crate::game::game_communicator::GameCommunicator;
use crate::game::id_types::{location_ids, LocationId, PlayerId, PromptInstanceId, ServerInstanceId, TokenInstanceId};
use crate::game::id_types::PlayerId::{Player1, Player2};
use crate::game::instruction::InstructionToClient;
use crate::game::player::Player;
use crate::game::prompts::{PromptCallback, PromptCallbackClosure, PromptInstance, PromptCallbackResult, PromptProfile, PromptType};
use crate::game::state_resources::StateResources;
use crate::game::tag::get_tag;
use crate::game::game_context::{ContextValue, GameContext};
use crate::game::new_state_machine::StateMachine;

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
            "move_card" => {
                let token_instance_id = get_tag("card", data)?.parse::<TokenInstanceId>()?;
                let target_location_id = get_tag("location", data)?.parse::<LocationId>()?;
                if resources.can_player_summon_token(token_instance_id, target_location_id, &mut communicator).await? {
                    state.summon_token(token_instance_id, target_location_id);
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
            let callback = resources.show_selectable_cards(&mut communicator).await?;
            callback.create_instructions(&mut communicator).await?;
            current_callback = Some(callback);
        }
    }
}

// pub async fn player_moved_card(&mut self, data: &str, communicator: &mut GameCommunicator) -> Result<Option<PromptCallback>> {
//     let card_id = get_tag("card", data)?.parse::<TokenInstanceId>()?;
//     let target_location_id = get_tag("location", data)?.parse::<LocationId>()?;
//
//     let card_instance = self.resources.card_instances.get(&card_id).context("Unable to find card")?;
//     let card_location = card_instance.location.clone();
//
//     let new_player_thaum = self.get_player(card_instance.owner).thaum - card_instance.cost;
//     let player = card_instance.owner;
//
//     let queue = self.resources.pre_move_card(card_id, target_location_id, self.current_turn, communicator).await?;
//     let behavior_result = card_behaviors::trigger_all_card_behaviors(
//         queue,
//         self.current_turn,
//         self,
//         communicator
//     ).await?;
//
//     if behavior_result == CardBehaviorResult::Cancel {
//         communicator.send_game_instruction( InstructionToClient::MoveCard { card: card_id, to: card_location }).await?;
//         return Ok(None)
//     }
//
//     self.get_player_mut(player).set_thaum(new_player_thaum, communicator).await?;
//
//     let queue = self.resources.move_card(card_id, target_location_id, self.current_turn, None, communicator).await?;
//     card_behaviors::trigger_all_card_behaviors(
//         queue,
//         self.current_turn,
//         self,
//         communicator
//     ).await?;
//     Ok(None)
// }