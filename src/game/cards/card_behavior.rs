use std::collections::VecDeque;
use color_eyre::eyre::{ContextCompat, eyre};
use toml::Table;
use crate::game::game_state::{CardKey, GameState, PlayerId};

use color_eyre::Result;
use crate::game::game_communicator::GameCommunicator;
use crate::game::instruction::Instruction;
use crate::game::trigger_context::{ContextValue, TriggerContext};

#[derive(Clone)]
pub struct Behavior {
    triggers: Vec<(TriggerDefinition, Table)>,
    actions: Vec<(BehaviourAction, Table)>,
}

impl Behavior {
    pub fn from(table: Table) -> Result<Self> {
        let mut triggers = Vec::new();
        let mut actions = Vec::new();

        for (key, value) in table {
            match key.as_str() {
                "trigger" => {
                    for trigger in value.as_array().ok_or_else(|| eyre!("Trigger is not an array"))? {
                        let trigger_table = trigger.as_table().context("Trigger is not a table")?;
                        triggers.push(BehaviorTrigger::from(trigger_table)?);
                    }
                }
                "action" => {
                    for action in value.as_array().ok_or_else(|| eyre!("Action is not an array"))? {
                        let action_table = action.as_table().context("Action is not a table")?;
                        actions.push(get_function(action_table)?);
                    }
                }
                _ => {}
            }
        }

        Ok(Self {
            triggers,
            actions,
        })
    }

    pub fn trigger(&self, trigger: BehaviorTrigger, player: PlayerId, context: &TriggerContext, state: &GameState, comm: &mut GameCommunicator, card: CardKey) -> Result<()> {
        let c = state.card_instances.get(card).ok_or_else(|| eyre!("Card not found during trigger"))?;

        for (t, m) in &self.triggers {
            if t.0 != trigger { continue; }
            if match t.1 {
                TriggerTarget::You => c.owner == player,
                TriggerTarget::Opponent => c.owner != player,
                TriggerTarget::Either => true,
                TriggerTarget::This => {
                    c.instance_id == context.get("iid").context("context does not contain iid")?.as_u64().context("iid is not a u64")?
                }
            } == false { continue; }

            for (key, value) in m {
                let mut modifiers = key.split('-').collect::<VecDeque<&str>>();

                if modifiers.len() == 0 {
                    return Err(eyre!("Invalid trigger key. Key is empty"));
                }

                let name = modifiers.pop_front().unwrap();

                if context.contains_key(name) == false {
                    return Err(eyre!("Trigger context does not contain key {}", name));
                }

                let context_value = &context.get(name).unwrap();

                let mut passed = match context_value {
                    ContextValue::String(s) => {
                        let my_val = value.as_str().context("Trigger value is not a string")?;
                        if modifiers.contains(&"contains") {
                            s.contains(my_val)
                        } else {
                            s == my_val
                        }
                    }
                    ContextValue::I64(i) => {
                        let my_val = value.as_integer().context("Trigger value is not an integer")?;
                        println!("{key} {} {}", i, my_val);
                        if key == "iid" && my_val == 0 { true } else {
                            if modifiers.contains(&"greater") {
                                *i > my_val
                            } else if modifiers.contains(&"less") {
                                *i < my_val
                            } else {
                                *i == my_val
                            }
                        }
                    }
                    ContextValue::F64(f) => {
                        let my_val = value.as_float().context("Trigger value is not a float")?;
                        if modifiers.contains(&"greater") {
                            *f > my_val
                        } else if modifiers.contains(&"less") {
                            *f < my_val
                        } else {
                            *f == my_val
                        }
                    }
                    ContextValue::Bool(b) => {
                        let my_val = value.as_bool().context("Trigger value is not a boolean")?;
                        *b == my_val
                    }
                    ContextValue::Array(a) => {
                        let my_val = value.as_array().context("Trigger value is not an array")?;
                        let my_val = my_val.iter().map(|x| x.as_str().unwrap().into()).collect::<Vec<String>>();
                        if modifiers.contains(&"contains") {
                            let mut passed = false;
                            for v in a {
                                if my_val.contains(v.as_string().unwrap()) {
                                    passed = true;
                                    break;
                                }
                            }
                            passed
                        } else {
                            let mut passed = true;
                            for v in a {
                                if !my_val.contains(v.as_string().unwrap()) {
                                    passed = false;
                                    break;
                                }
                            }
                            passed
                        }
                    }
                    _ => {
                        return Err(eyre!("Trigger value is not a valid type"));
                    }
                };
                if modifiers.contains(&"not") {
                    passed = !passed;
                }

                if !passed {
                    return Ok(());
                }
            }

            for (action, inputs) in &self.actions {
                (action)(state, comm, card, inputs)?;
            }
        }

        Ok(())
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum BehaviorTrigger {
    TurnStart,
    TurnEnd,
    DrawCard,
    Summon,
    EnterLandscape,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum TriggerTarget {
    You,
    Opponent,
    Either,
    This,
}

type TriggerDefinition = (BehaviorTrigger, TriggerTarget);

impl BehaviorTrigger {
    pub fn from(trigger: &Table) -> Result<(TriggerDefinition, Table)> {
        let trigger_type = trigger.get("type").context("Trigger type is undefined")?.as_str().context("Trigger type is not a string")?;
        let inputs = trigger.get("match");
        let inputs = match inputs {
            Some(inputs) => inputs.as_table().context("Trigger inputs is not a table")?.to_owned(),
            None => Table::new()
        };

        // try to split at hyphen
        let splits = trigger_type.split("-").collect::<Vec<&str>>();

        let trigger = match splits.last().unwrap().clone() {
            "turn_start" => Self::TurnStart,
            "turn_end" => Self::TurnEnd,
            "draw_card" => Self::DrawCard,
            "summon" => Self::Summon,
            "enter_landscape" => Self::EnterLandscape,
            _ => return Err(eyre!("Invalid trigger type {}", trigger_type)),
        };

        let who = if splits.len() > 1 {
            match splits[0] {
                "you" => TriggerTarget::You,
                "opponent" => TriggerTarget::Opponent,
                "either" => TriggerTarget::Either,
                "this" => TriggerTarget::This,
                _ => return Err(eyre!("Invalid trigger target {}", splits[0])),
            }
        } else {
            TriggerTarget::Either
        };

        Ok(((trigger, who), inputs))
    }
}

pub type BehaviourAction = fn(&GameState, &mut GameCommunicator, CardKey, &Table) -> Result<()>;

pub fn get_function(action: &Table) -> Result<(BehaviourAction, Table)> {
    let action_type = action.get("type").context("Action type is undefined")?.as_str().context("Action type is not a string")?;
    let inputs = action.get("inputs");
    let inputs = match inputs {
        Some(inputs) => inputs.as_table().context("Action inputs is not a table")?.to_owned(),
        None => Table::new()
    };

    Ok((match action_type {
        "you_draw_card" => you_draw_card,
        "replace_this" => replace_this,
        "replace_group" => replace_group,
        _ => return Err(eyre!("Unknown action type: {}", action_type)),
    }, inputs))
}

pub fn you_draw_card(state: &GameState, communicator: &mut GameCommunicator, card: CardKey, _table: &Table) -> Result<()> {
    let card = state.card_instances.get(card).unwrap();
    state.get_player(card.owner).draw_card(state, communicator)?;
    Ok(())
}

pub fn replace_this(state: &GameState, communicator: &mut GameCommunicator, card: CardKey, table: &Table) -> Result<()> {
    let with = table.get("with").context("Replace action has no 'with' input")?.as_str().context("Replace action 'with' input is not a string")?;
    communicator.queue.enqueue(Instruction::Destroy { card });
    let card_instance = state.card_instances.get(card).unwrap();
    communicator.queue.enqueue(Instruction::CreateCard { id: with.into(), iid: fastrand::u64(..), location: card_instance.location, player: card_instance.owner });
    Ok(())
}

pub fn replace_group(state: &GameState, communicator: &mut GameCommunicator, card: CardKey, table: &Table) -> Result<()> {
    let count = table.get("count").context("Count is undefined")?.as_integer().context("Count is not an integer")? as usize;
    let id = table.get("id").context("ID is undefined")?.as_str().context("ID is not a string")?;
    let replace_with = table.get("replace_with").context("Replace with is undefined")?.as_str().context("Replace with is not a string")?;
    let owner = table.get("owner").context("Owner is undefined")?.as_str().context("Owner is not a string")?;

    let this_card = state.card_instances.get(card).unwrap();

    let mut found = Vec::new();
    found.push(card);

    for key in state.board.get_cards_in_play(state) {
        let card = state.card_instances.get(key).unwrap();
        if card.card_id != id { continue; }
        if card.instance_id == this_card.instance_id { continue; }
        if card.is_alive(state) == false { continue; }
        match owner {
            "you" => {
                if card.owner == this_card.owner {
                    found.push(key);
                }
            }
            "opponent" => {
                if card.owner != this_card.owner {
                    found.push(key);
                }
            }
            "either" => {
                found.push(key);
            }
            &_ => {}
        }

        if found.len() >= count { break; }
    }

    if found.len() >= count {
        for i in 0..count { communicator.queue.enqueue(Instruction::Destroy { card: found[i] }); }
        communicator.queue.enqueue(Instruction::CreateCard {
            id: replace_with.into(),
            iid: fastrand::u64(..),
            location: this_card.location,
            player: this_card.owner,
        });
    }

    Ok(())
}