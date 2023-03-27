use std::ops::Not;
use async_recursion::async_recursion;

use color_eyre::eyre::{Context, ContextCompat, Error};
use color_eyre::Result;

use crate::game::cards::token_deserializer::{CardBehaviorAction, CardBehaviorTriggerWhenActivator, CardBehaviorTriggerWhenName};
use crate::game::game_communicator::GameCommunicator;
use crate::game::id_types::{TokenInstanceId, PlayerId};
use crate::game::new_state_machine::StateMachine;
use crate::game::state_resources::StateResources;
use crate::game::game_context::{context_keys, GameContext};

#[derive(Clone, PartialEq, Debug)]
pub enum CardBehaviorResult {
    Ok,
    Cancel,
}

pub async fn trigger_card_behaviors(card_instance_id: TokenInstanceId, trigger_name: CardBehaviorTriggerWhenName, context: &mut GameContext, state: &mut StateMachine, resources: &mut StateResources, communicator: &mut GameCommunicator) -> Result<()> {
    let card = resources.token_instances.get(&card_instance_id).context(format!("Tried to process behaviors for card that does not exist: {}", card_instance_id))?;

    let is_owned = card.owner == context.get(context_keys::OWNER)?.as_player_id()?;
    let is_context_this = context.get(context_keys::TRIGGER_THIS).map_or(
        Ok::<bool, Error>(false),
        |card_instance_value| {
            let instance_id = card_instance_value.as_token_instance_id()?;
            Ok(instance_id == card_instance_id)
        }
    )?;

    let mut final_result = CardBehaviorResult::Ok;

    for behavior in &card.behaviors.clone() { // Todo: Is clone required here? I assume so. Ask Marc
        // Check if a trigger passed
        let mut successful_triggers = Vec::new();
        for trigger in &behavior.triggers {
            // Match activator
            if match trigger.when.activator {
                CardBehaviorTriggerWhenActivator::Owned => is_owned,
                CardBehaviorTriggerWhenActivator::Opponent => is_owned.not(),
                CardBehaviorTriggerWhenActivator::This => is_context_this,
                CardBehaviorTriggerWhenActivator::Either => true
            }.not() { continue; }

            if trigger.when.name != trigger_name {
                continue;
            }

            if let Some(and) = &trigger.and {
                if and.check(context, resources, communicator).await? == false { continue; }
            }

            successful_triggers.push(trigger);
        }

        if successful_triggers.len() > 0 {
            for action in &behavior.actions {
                let result= action.run(context, resources, state, communicator).await?;
            }
        }
    }

    Ok(())
}