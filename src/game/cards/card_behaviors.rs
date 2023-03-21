use std::ops::Not;

use color_eyre::eyre::{ContextCompat, Error};
use color_eyre::Result;

use crate::game::cards::card_deserialization::{CardBehaviorAction, CardBehaviorTriggerWhenActivator, CardBehaviorTriggerWhenName};
use crate::game::game_communicator::GameCommunicator;
use crate::game::game_state::{CardBehaviorTriggerQueue, GameState};
use crate::game::id_types::{CardInstanceId, PlayerId};
use crate::game::trigger_context::CardBehaviorContext;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum CardBehaviorResult {
    Ok,
    Cancel
}

pub async fn trigger_all_card_behaviors(mut queue: CardBehaviorTriggerQueue, trigger_owner: PlayerId, state: &mut GameState, communicator: &mut GameCommunicator) -> Result<CardBehaviorResult> {
    let mut final_result = CardBehaviorResult::Ok;

    while let Some((trigger_when, trigger_context)) = queue.pop_front() {
        for (card_instance_id, _) in state.resources.card_instances.clone() {
            let (mut trigger_queue, result) = trigger_card_behaviors(
                card_instance_id,
                trigger_context.owner.clone(),
                trigger_when.clone(),
                &trigger_context,
                state,
                communicator
            ).await?;
            queue.append(&mut trigger_queue);
            final_result = match result {
                CardBehaviorResult::Ok => final_result,
                CardBehaviorResult::Cancel => CardBehaviorResult::Cancel
            };
        }
    }

    Ok(final_result)
}

pub async fn trigger_card_behaviors(card_instance_id: CardInstanceId, trigger_owner: PlayerId, trigger_name: CardBehaviorTriggerWhenName, context: &CardBehaviorContext, state: &mut GameState, communicator: &mut GameCommunicator) -> Result<(CardBehaviorTriggerQueue, CardBehaviorResult)> {
    let card = state.resources.card_instances.get(&card_instance_id).context(format!("Tried to process behaviors for card that does not exist: {}", card_instance_id))?;

    let is_owned = card.owner == trigger_owner;
    let is_context_this = context.get("card_instance").map_or(
        Ok::<bool, Error>(false),
        |card_instance_value| {
            let instance_id = card_instance_value.as_card_instance().context("Given key \"card_instance\" in trigger context was not a CardInstance")?;
            Ok(instance_id == card_instance_id)
        }
    )?;

    let mut queue = CardBehaviorTriggerQueue::new();

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
                if and.check(context, state, communicator).await? == false { continue; }
            }

            successful_triggers.push(trigger);
        }

        if successful_triggers.len() > 0 {
            for action in &behavior.actions {
                let (mut new_queue, result) = action.run(&context, state, communicator).await?;
                queue.append(&mut new_queue);
                final_result = match result {
                    CardBehaviorResult::Ok => final_result,
                    CardBehaviorResult::Cancel => CardBehaviorResult::Cancel
                }
            }
        }
    }

    Ok((queue, final_result))
}