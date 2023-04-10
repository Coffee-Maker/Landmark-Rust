use std::fmt;
use color_eyre::eyre::{Context, ContextCompat};

use color_eyre::Result;
use serde::{de, Deserialize, Deserializer};
use serde::__private::de::EnumDeserializer;
use serde::de::{Error, MapAccess, Unexpected, Visitor};
use serde::de::value::StringDeserializer;
use serde_enum_str::Deserialize_enum_str;
use crate::game::tokens;

use crate::game::tokens::token_behaviors::TokenBehaviorResult;
use crate::game::tokens::token_instance::TokenInstance;
use crate::game::game_communicator::GameCommunicator;
use crate::game::id_types::{TokenInstanceId, location_ids, PlayerId};
use crate::game::id_types::PlayerId::{Player1, Player2};
use crate::game::instruction::InstructionToClient;
use crate::game::new_state_machine::StateMachine;
use crate::game::player::Player;
use crate::game::state_resources::StateResources;
use crate::game::game_context::{context_keys, GameContext};

#[derive(Deserialize, Debug, Clone)]
pub struct TokenData {
    #[serde(default)] pub nightly: bool,
    #[serde(skip_deserializing)] pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub cost: u32,
    pub types: Vec<String>,

    #[serde(flatten)]
    pub token_category: TokenCategory,

    #[serde(rename = "behavior", default)]
    pub behaviors: Vec<TokenBehavior>,
}

#[derive(Deserialize, Debug, PartialEq, Clone, Copy)]
pub struct SlotPosition {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl SlotPosition {
    pub fn is_adjacent_to(&self, other: SlotPosition) -> bool {
        let delta_x = (self.x - other.x).abs();
        let delta_y = (self.y - other.y).abs();
        let delta_z = (self.z - other.z).abs();
        delta_x + delta_y + delta_z == 1
    }
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "snake_case", tag = "category")]
pub enum TokenCategory {
    Hero {
        health: i32,
        #[serde(default)] defense: i32
    },
    Landscape {
        slots: Vec<SlotPosition>
    },
    Unit {
        health: i32,
        #[serde(default)] attack: i32,
        #[serde(default)] defense: i32,
    },
    Item,
    Command,
}

#[derive(Deserialize, Debug, Clone)]
pub struct TokenBehavior {
    pub name: Option<String>,
    pub description: Option<String>,

    #[serde(rename = "trigger")]
    pub triggers: Vec<TokenBehaviorTrigger>,

    #[serde(rename = "action")]
    pub actions: Vec<TokenBehaviorAction>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct TokenBehaviorTrigger {
    pub when: TokenBehaviorTriggerWhen,
    pub and: Option<TokenBehaviorTriggerAnd>
}

#[derive(Debug, Clone)]
pub struct TokenBehaviorTriggerWhen {
    pub activator: TokenBehaviorTriggerWhenActivator,
    pub name: TokenBehaviorTriggerWhenName,
}

#[derive(Debug, Deserialize_enum_str, Clone)]
#[serde(rename_all = "snake_case")]
pub enum TokenBehaviorTriggerWhenActivator {
    #[serde(alias = "owner")]
    Owned,

    Opponent,
    This,
    Either
}

#[derive(Debug, Deserialize_enum_str, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TokenBehaviorTriggerWhenName {
    // Generic
    TurnStarted, // Todo
    TurnEnded, // Todo

    // Token
    HasBeenCreated,
    WillDrawToken,
    HasDrawnToken,
    HasBeenDrawn,
    WillDestroy,
    WillBeDestroyed,
    HasDestroyed,
    HasBeenDestroyed,
    WillBeMoved,
    HasBeenMoved,

    // Units
    WillBeSummoned,
    HasBeenSummoned,
    WillAttack,
    WillBeAttacked,
    HasAttacked,
    HasBeenAttacked,
    TookDamage,
    WillBeEffectDamaged,
    HasBeenEffectDamaged,
    WillDefeat,
    WillBeDefeated,
    HasDefeated,
    HasBeenDefeated,
    WillLeaveLandscape,
    HasLeftLandscape,
    WillEnterLandscape,
    HasEnteredLandscape,
    WillEquip,
    HasEquipped,

    // Items
    WillBeEquipped,
    HasBeenEquipped,

    // Commands
    WillCast,
    HasCast,

    // Misc (Internal)
    CheckCancel,
}

impl<'de> Deserialize<'de> for TokenBehaviorTriggerWhen {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct TokenBehaviorTriggerWhenVisitor;

        impl<'de> Visitor<'de> for TokenBehaviorTriggerWhenVisitor {
            type Value = TokenBehaviorTriggerWhen;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("\"<target>:<trigger>\"")
            }

            fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
                let [activator, name] = v.split(':').collect::<Vec<_>>()[..] else {
                    return Err(de::Error::invalid_value(Unexpected::Str(v), &self))
                };

                Ok(TokenBehaviorTriggerWhen {
                    activator: activator.parse::<TokenBehaviorTriggerWhenActivator>()
                        .map_err(|e| serde::de::Error::custom(e))?,
                    name: name.parse::<TokenBehaviorTriggerWhenName>()
                        .map_err(|e| serde::de::Error::custom(e))?,
                })
            }
        }

        deserializer.deserialize_string(TokenBehaviorTriggerWhenVisitor)
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case", tag = "check", content = "with")]
pub enum TokenBehaviorTriggerAnd {
    TypeContains {
        target: TokenTarget,
        types: Vec<String>,
    },
    Count {
        filter: TokenFilter,
        condition: CountCondition,
        count: i32
    },
    AdjacentTo {
        source: UnitTarget,
        target: UnitTarget
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum CountCondition {
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Equal,
    NotEqual
}

impl CountCondition {
    pub fn evaluate(&self, a: i32, b: i32) -> bool {
        match self {
            CountCondition::Greater => a > b,
            CountCondition::GreaterEqual => a >= b,
            CountCondition::Less => a < b,
            CountCondition::LessEqual => a <= b,
            CountCondition::Equal => a == b,
            CountCondition::NotEqual => a != b,
        }
    }
}

impl TokenBehaviorTriggerAnd {
    pub async fn check(&self, context: &GameContext, resources: &mut StateResources, communicator: &mut GameCommunicator) -> Result<bool> {
        Ok(match self {
            TokenBehaviorTriggerAnd::TypeContains { target, types } => {
                let targets = target.evaluate(context, resources)?;
                let mut passed = true;
                'outer: for target in targets {
                    let target_instance = resources.token_instances.get(&target).context(format!("Token with the id {target} was not a found in state resources"))?;
                    for t in types {
                        if target_instance.token_types.contains(t) == false {
                            passed = false;
                            break 'outer;
                        }
                    }
                }
                passed
            }
            TokenBehaviorTriggerAnd::Count { filter, condition, count } => {
                let mut tokens = resources.token_instances.values().collect::<Vec<&TokenInstance>>();
                filter.evaluate(&mut tokens, &context, resources);
                condition.evaluate(tokens.len() as i32, *count)
            }
            TokenBehaviorTriggerAnd::AdjacentTo { source, target } => {
                let mut passed = false;
                for source_id in source.evaluate(context, resources)? {
                    for target_id in target.evaluate(context, resources)? {
                        let source = resources.token_instances.get(&source_id).unwrap();
                        let target = resources.token_instances.get(&target_id).unwrap();
                        let source_slot = location_ids::get_slot_position(source.location, &resources.board);
                        let target_slot = location_ids::get_slot_position(target.location, &resources.board);
                        if let Ok(source_slot) = source_slot {
                            if let Ok(target_slot) = target_slot {
                                if source_slot.is_adjacent_to(target_slot) {
                                    passed = true;
                                    break;
                                }
                            }
                        }
                    }
                    if passed { break; }
                }
                passed
            }
        })
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum PlayerTarget {
    Owner,
    Opponent,
    Either,
    Random,
}

impl Default for PlayerTarget {
    fn default() -> Self {
        Self::Either
    }
}

impl PlayerTarget {
    pub fn evaluate(&self, owner: PlayerId) -> Vec<PlayerId> {
        match self {
            PlayerTarget::Owner => vec!(owner),
            PlayerTarget::Opponent => {
                match owner {
                    PlayerId::Player1 => vec!(Player2),
                    PlayerId::Player2 => vec!(Player1),
                }
            }
            PlayerTarget::Either => vec!(Player1, Player2),
            PlayerTarget::Random => if fastrand::bool() { vec!(Player1) } else { vec!(Player2) }
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum UnitTarget {
    This,
    Find {
        filter: Box<TokenFilter>
    },
    EquippingUnit,
    All,
    Context {
        key: String
    }
}

impl UnitTarget {
    pub fn evaluate(&self, context: &GameContext, resources: &StateResources) -> Result<Vec<TokenInstanceId>> {
        Ok(match self {
            UnitTarget::This => vec!(context.get(context_keys::ACTION_THIS)?.as_token_instance_id()?),
            UnitTarget::Find { filter } => {
                let mut tokens = resources.token_instances.values().collect::<Vec<&TokenInstance>>();
                filter.evaluate(&mut tokens, context, resources)?;
                tokens.iter().map(|c| c.instance_id).collect::<Vec<TokenInstanceId>>()
            }
            UnitTarget::EquippingUnit => {
                let this_id = context.get(context_keys::ACTION_THIS)?.as_token_instance_id()?;
                let this_instance = resources.token_instances.get(&this_id).unwrap();
                let equipping_unit = resources.equipment_slot_owners.get(&this_instance.location).context("Item is not in equipment slot")?;
                vec!(*equipping_unit)
            }
            UnitTarget::All => todo!(),
            UnitTarget::Context { key } => {
                let value = context.get(key)?.as_token_instance_id()?;
                vec!(context.get(key)?.as_token_instance_id()?)
            },
        })
    }
}

impl Default for UnitTarget {
    fn default() -> Self {
        Self::All
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum TokenTarget {
    This,
    Find {
        filter: TokenFilter
    },
    EquippingUnit,
    Context {
        key: String
    }
}

impl TokenTarget {
    pub fn evaluate(&self, context: &GameContext, resources: &StateResources) -> Result<Vec<TokenInstanceId>> {
        Ok(match self {
            TokenTarget::This => vec!(context.get(context_keys::ACTION_THIS)?.as_token_instance_id()?),
            TokenTarget::Find { filter } => {
                let mut tokens = resources.token_instances.values().collect::<Vec<&TokenInstance>>();
                filter.evaluate(&mut tokens, context, resources)?;
                tokens.iter().map(|c| c.instance_id).collect::<Vec<TokenInstanceId>>()
            },
            TokenTarget::EquippingUnit => {
                let this_id = context.get(context_keys::ACTION_THIS)?.as_token_instance_id()?;
                let this_instance = resources.token_instances.get(&this_id).unwrap();
                let equipping_unit = resources.equipment_slot_owners.get(&this_instance.location).context("Item is not in equipment slot")?;
                vec!(*equipping_unit)
            }
            TokenTarget::Context { key } => vec!(context.get(key)?.as_token_instance_id()?),
        })
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum LocationTarget {
    OwnerHand,
    OwnerSet,
    OwnerGraveyard,
    OpponentHand,
    OpponentSet,
    OpponentGraveyard
}

#[derive(Deserialize, Debug, Clone)]
pub struct TokenFilter {
    owned_by: Option<PlayerTarget>,
    adjacent_to: Option<UnitTarget>,
    contains_types: Option<Vec<String>>,
    id_is: Option<Vec<String>>,
}

impl TokenFilter {
    pub fn evaluate(&self, tokens: &mut Vec<&TokenInstance>, context: &GameContext, resources: &StateResources) -> Result<()> {
        if let Some(owned_by) = &self.owned_by {
            tokens.retain(|c| owned_by.evaluate(context.get(context_keys::OWNER).unwrap().as_player_id().unwrap()).contains(&c.owner))
        }

        if let Some(adjacent_to) = &self.adjacent_to {
            tokens.retain(|c| {
                let position = location_ids::get_slot_position(c.location, &resources.board);
                if let Ok(position) = position {
                    for token_to_check in adjacent_to.evaluate(context, resources).unwrap() { // I don't know what to do here
                        let check_token_instance = resources.token_instances.get(&token_to_check);
                        if let Some(check_token_instance) = check_token_instance {
                            let check_slot_pos = location_ids::get_slot_position(check_token_instance.location, &resources.board);
                            if let Ok(check_slot_pos) = check_slot_pos {
                                if check_slot_pos.is_adjacent_to(position) {
                                    return true;
                                }
                            }
                        }
                    }
                }
                // Todo: Leaving that monstrosity there as a welcome back for Marc
                false
            });
        }

        if let Some(contains_types) = &self.contains_types {
            tokens.retain(|c| {
                for t in contains_types {
                    return c.token_types.contains(t);
                }
                false
            });
        }

        if let Some(id_is) = &self.id_is {
            tokens.retain(|c| {
                for id in id_is {
                    return c.token_data.id == *id;
                }
                false
            })
        }
        Ok(())
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case", tag = "then", content = "with")]
pub enum TokenBehaviorAction {
    DrawToken {
        target: PlayerTarget,
    },
    Replace {
        target: UnitTarget,
        replacement: String,
    },
    AddTypes {
        target: UnitTarget,
        types: Vec<String>,
    },
    ModifyAttack {
        target: UnitTarget,
        amount: i32,
    },
    ModifyHealth {
        target: UnitTarget,
        amount: i32,
    },
    ModifyDefense {
        target: UnitTarget,
        amount: i32,
    },
    ModifyCost {
        target: UnitTarget,
        amount: i32,
    },
    Destroy {
        target: TokenTarget,
    },
    Summon {
        target: UnitTarget,
        token: String,
    },

    // New
    RedirectTarget {
        new_target: UnitTarget,
    },
    DamageUnit {
        target: UnitTarget,
        amount: u32,
    },
    DamageHero {
        target: PlayerTarget,
        amount: u32,
    },
    GiveAllTypes {
        target: TokenTarget
    },
    Cancel,
    SelectUnit {
        context_key: String,
        filter: TokenFilter,
    },
    SaveContext {
        context_key: String,
        personal_key: String,
    },
    SumAttack {
        target: UnitTarget,
        filter: TokenFilter,
    },
    AddBehavior {
        target: TokenTarget,
        behavior: String,
    },
    RemoveBehavior {
        target: TokenTarget,
        behavior: String,
    },
    SetCounter {
        target: TokenTarget,
        counter: String,
        value: i32,
    },
    ModifyCounter {
        target: TokenTarget,
        counter: String,
        amount: i32,
    },
    CreateToken {
        location: LocationTarget,
        id: String,
    }
}

impl TokenBehaviorAction {
    pub async fn run(&self, context: &mut GameContext, resources: &mut StateResources, state: &mut StateMachine, communicator: &mut GameCommunicator) -> Result<TokenBehaviorResult> {
        let this = context.get(context_keys::ACTION_THIS)?.as_token_instance_id()?;

        let result = match self {
            TokenBehaviorAction::DrawToken { target } => {
                for player in target.evaluate(context.get(context_keys::OWNER)?.as_player_id()?) {
                    state.draw_token(player);
                }
                TokenBehaviorResult::Ok
            }
            TokenBehaviorAction::Replace { target, replacement } => {
                let target = target.evaluate(&context, resources)?;
                for token_instance_id in &target {
                    let token_instance = resources.token_instances.get(token_instance_id).context("Tried to replace a token that does not exist")?;
                    let location = token_instance.location;
                    let owner = token_instance.owner;
                    state.create_token(replacement, owner, location);
                    state.destroy_token(this, *token_instance_id);
                }

                TokenBehaviorResult::Ok
            },
            TokenBehaviorAction::AddTypes { target, types } => todo!(),
            TokenBehaviorAction::ModifyAttack { target, amount } => {
                for target in target.evaluate(context, resources)? {
                    let target_instance = resources.token_instances.get_mut(&target).unwrap();
                    target_instance.current_stats.attack += amount;
                    communicator.send_game_instruction(InstructionToClient::UpdateData { token_data: target_instance.clone() }).await?;
                }
                TokenBehaviorResult::Ok
            },
            TokenBehaviorAction::ModifyHealth { target, amount } => {
                for target in target.evaluate(context, resources)? {
                    let target_instance = resources.token_instances.get_mut(&target).unwrap();
                    target_instance.current_stats.health += amount;
                    communicator.send_game_instruction(InstructionToClient::UpdateData { token_data: target_instance.clone() }).await?;
                }
                TokenBehaviorResult::Ok
            },
            TokenBehaviorAction::ModifyDefense { target, amount }  => {
                for target in target.evaluate(context, resources)? {
                    let target_instance = resources.token_instances.get_mut(&target).unwrap();
                    target_instance.current_stats.defense += amount;
                    communicator.send_game_instruction(InstructionToClient::UpdateData { token_data: target_instance.clone() }).await?;
                }
                TokenBehaviorResult::Ok
            },
            TokenBehaviorAction::ModifyCost { target, amount } => {
                for target in target.evaluate(context, resources)? {
                    let target_instance = resources.token_instances.get_mut(&target).unwrap();
                    if target_instance.cost as i32 + amount < 0 {
                        target_instance.cost = 0;
                    } else {
                        target_instance.cost = (target_instance.cost as i32 + amount) as u32;
                    }
                    communicator.send_game_instruction(InstructionToClient::UpdateData { token_data: target_instance.clone() }).await?;
                }
                TokenBehaviorResult::Ok
            },
            TokenBehaviorAction::Destroy { target } => {
                if let Ok(tokens) = target.evaluate(context, resources) {
                    for token in tokens {
                        state.destroy_token(this, token);
                    }
                }
                TokenBehaviorResult::Ok
            }
            TokenBehaviorAction::Summon { target, token } => todo!(),

            TokenBehaviorAction::GiveAllTypes { .. } => todo!(),
            TokenBehaviorAction::Cancel => TokenBehaviorResult::Cancel,
            TokenBehaviorAction::SelectUnit { .. } => todo!(),
            TokenBehaviorAction::SaveContext { .. } => todo!(),
            TokenBehaviorAction::SumAttack { target, filter } => todo!(),
            TokenBehaviorAction::AddBehavior { .. } => todo!(),
            TokenBehaviorAction::RemoveBehavior { .. } => todo!(),
            TokenBehaviorAction::SetCounter { .. } => todo!(),
            TokenBehaviorAction::ModifyCounter { .. } => todo!(),
            TokenBehaviorAction::CreateToken { .. } => todo!(),
            TokenBehaviorAction::DamageHero { target, amount } => {
                for target in target.evaluate(context.get(context_keys::OWNER)?.as_player_id()?) {
                    state.deal_effect_damage(this, resources.get_player(target).hero, *amount as i32);
                }
                TokenBehaviorResult::Ok
            },
            TokenBehaviorAction::DamageUnit { target, amount } => {
                for token_instance_id in target.evaluate(context, resources)? {
                    state.deal_effect_damage(this, token_instance_id, *amount as i32);
                }
                TokenBehaviorResult::Ok
            },
            TokenBehaviorAction::RedirectTarget { new_target } => {
                let new_target = new_target.evaluate(context, resources)?;
                let new_target = new_target.first();
                if let Some(new_target) = new_target {
                    todo!();
                }
                TokenBehaviorResult::Ok
            }
        };

        Ok(result)
    }
}