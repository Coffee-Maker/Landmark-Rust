use std::ops::Not;

use color_eyre::eyre::{ContextCompat, Error};
use color_eyre::Result;

use crate::game::cards::card_deserialization::{CardBehaviorAction, CardBehaviorTriggerWhenActivator, CardBehaviorTriggerWhenName};
use crate::game::game_communicator::GameCommunicator;
use crate::game::game_state::{CardBehaviorTriggerQueue, GameState};
use crate::game::id_types::{CardInstanceId, PlayerId};
use crate::game::trigger_context::CardBehaviorTriggerContext;

pub fn trigger_card_behaviors(card_instance_id: CardInstanceId, trigger_owner: PlayerId, trigger_name: CardBehaviorTriggerWhenName, context: &CardBehaviorTriggerContext, state: &mut GameState, communicator: &mut GameCommunicator) -> Result<CardBehaviorTriggerQueue> {
    let card = state.resources.card_instances.get(&card_instance_id).context(format!("Tried to process behaviours for card that does not exist: {}", card_instance_id))?;

    let is_owned = card.owner == trigger_owner;
    let is_context_this = context.get("card_instance").map_or(
        Ok::<bool, Error>(false),
        |card_instance_value| {
            let instance_id = card_instance_value.as_card_instance().context("Given key \"card_instance\" in trigger context was not a CardInstance")?;
            Ok(instance_id == card_instance_id)
        }
    )?;

    let mut queue = CardBehaviorTriggerQueue::new();

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

            successful_triggers.push(trigger);
        }

        if successful_triggers.len() > 0 {
            for action in &behavior.actions {
                queue.append(&mut process_behavior_action(&action, &context, state, communicator)?);
            }
        }
    }

    Ok(queue)
}

fn process_behavior_action(action: &CardBehaviorAction, context: &CardBehaviorTriggerContext, state: &mut GameState, communicator: &mut GameCommunicator) -> Result<CardBehaviorTriggerQueue> {
    match action {
        CardBehaviorAction::DrawCard { target } => todo!(),
        CardBehaviorAction::Replace { target, replacement } => todo!(),
        CardBehaviorAction::AddTypes { target, types } => todo!(),
        CardBehaviorAction::ModifyAttack { target, amount } => todo!(),
        CardBehaviorAction::ModifyHealth { target, amount } => todo!(),
        CardBehaviorAction::ModifyDefense { target, amount } => todo!(),
        CardBehaviorAction::ModifyCost { target, amount } => todo!(),
        CardBehaviorAction::Destroy { target } => todo!(),
        CardBehaviorAction::Summon { target, card } => todo!(),
    }
}