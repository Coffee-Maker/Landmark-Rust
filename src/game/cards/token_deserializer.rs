use std::fmt;
use color_eyre::eyre::{Context, ContextCompat};

use color_eyre::Result;
use serde::{de, Deserialize, Deserializer};
use serde::__private::de::EnumDeserializer;
use serde::de::{Error, MapAccess, Unexpected, Visitor};
use serde::de::value::StringDeserializer;
use serde_enum_str::Deserialize_enum_str;
use crate::game::cards;

use crate::game::cards::token_behaviors::CardBehaviorResult;
use crate::game::cards::card_instance::TokenInstance;
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
    pub triggers: Vec<CardBehaviorTrigger>,

    #[serde(rename = "action")]
    pub actions: Vec<CardBehaviorAction>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct CardBehaviorTrigger {
    pub when: CardBehaviorTriggerWhen,
    pub and: Option<CardBehaviorTriggerAnd>
}

#[derive(Debug, Clone)]
pub struct CardBehaviorTriggerWhen {
    pub activator: CardBehaviorTriggerWhenActivator,
    pub name: CardBehaviorTriggerWhenName,
}

#[derive(Debug, Deserialize_enum_str, Clone)]
#[serde(rename_all = "snake_case")]
pub enum CardBehaviorTriggerWhenActivator {
    #[serde(alias = "owner")]
    Owned,

    Opponent,
    This,
    Either
}

#[derive(Debug, Deserialize_enum_str, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CardBehaviorTriggerWhenName {
    // Generic
    TurnStarted, // Todo
    TurnEnded, // Todo

    // Card
    HasBeenCreated,
    WillDrawCard,
    HasDrawnCard,
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

impl<'de> Deserialize<'de> for CardBehaviorTriggerWhen {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct CardBehaviorTriggerWhenVisitor;

        impl<'de> Visitor<'de> for CardBehaviorTriggerWhenVisitor {
            type Value = CardBehaviorTriggerWhen;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("\"<target>:<trigger>\"")
            }

            fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
                let [activator, name] = v.split(':').collect::<Vec<_>>()[..] else {
                    return Err(de::Error::invalid_value(Unexpected::Str(v), &self))
                };

                Ok(CardBehaviorTriggerWhen {
                    activator: activator.parse::<CardBehaviorTriggerWhenActivator>()
                        .map_err(|e| serde::de::Error::custom(e))?,
                    name: name.parse::<CardBehaviorTriggerWhenName>()
                        .map_err(|e| serde::de::Error::custom(e))?,
                })
            }
        }

        deserializer.deserialize_string(CardBehaviorTriggerWhenVisitor)
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case", tag = "check", content = "with")]
pub enum CardBehaviorTriggerAnd {
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

impl CardBehaviorTriggerAnd {
    pub async fn check(&self, context: &GameContext, resources: &mut StateResources, communicator: &mut GameCommunicator) -> Result<bool> {
        Ok(match self {
            CardBehaviorTriggerAnd::TypeContains { target, types } => {
                let targets = target.evaluate(context, resources)?;
                let mut passed = true;
                'outer: for target in targets {
                    let target_instance = resources.token_instances.get(&target).context(format!("Card with the id {target} was not a found in state resources"))?;
                    for t in types {
                        if target_instance.card_types.contains(t) == false {
                            passed = false;
                            break 'outer;
                        }
                    }
                }
                passed
            }
            CardBehaviorTriggerAnd::Count { filter, condition, count } => {
                let mut cards = resources.token_instances.values().collect::<Vec<&TokenInstance>>();
                filter.evaluate(&mut cards, &context, resources);
                condition.evaluate(cards.len() as i32, *count)
            }
            CardBehaviorTriggerAnd::AdjacentTo { source, target } => {
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
    All,
    Context {
        key: String
    }
}

impl UnitTarget {
    pub fn evaluate(&self, context: &GameContext, resources: &StateResources) -> Result<Vec<TokenInstanceId>> {
        Ok(match self {
            UnitTarget::This => vec!(context.get(context_keys::TOKEN_INSTANCE)?.as_token_instance_id()?),
            UnitTarget::Find { filter } => {
                let mut cards = resources.token_instances.values().collect::<Vec<&TokenInstance>>();
                filter.evaluate(&mut cards, context, resources)?;
                cards.iter().map(|c| c.instance_id).collect::<Vec<TokenInstanceId>>()
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
    EquipTarget,
    Find {
        filter: TokenFilter
    },
    Context {
        key: String
    }
}

impl TokenTarget {
    pub fn evaluate(&self, context: &GameContext, resources: &StateResources) -> Result<Vec<TokenInstanceId>> {
        Ok(match self {
            TokenTarget::This => vec!(context.get(context_keys::TOKEN_INSTANCE)?.as_token_instance_id()?),
            TokenTarget::EquipTarget => todo!(),
            TokenTarget::Find { filter } => {
                let mut cards = resources.token_instances.values().collect::<Vec<&TokenInstance>>();
                filter.evaluate(&mut cards, context, resources)?;
                cards.iter().map(|c| c.instance_id).collect::<Vec<TokenInstanceId>>()
            },
            TokenTarget::Context { key } => vec!(context.get(key)?.as_token_instance_id()?),
        })
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum LocationTarget {
    OwnerHand,
    OwnerDeck,
    OwnerGraveyard,
    OpponentHand,
    OpponentDeck,
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
    pub fn evaluate(&self, cards: &mut Vec<&TokenInstance>, context: &GameContext, resources: &StateResources) -> Result<()> {
        if let Some(owned_by) = &self.owned_by {
            cards.retain(|c| owned_by.evaluate(context.get(context_keys::OWNER).unwrap().as_player_id().unwrap()).contains(&c.owner))
        }

        if let Some(adjacent_to) = &self.adjacent_to {
            cards.retain(|c| {
                let position = location_ids::get_slot_position(c.location, &resources.board);
                if let Ok(position) = position {
                    for card_to_check in adjacent_to.evaluate(context, resources).unwrap() { // I don't know what to do here
                        let check_card_instance = resources.token_instances.get(&card_to_check);
                        if let Some(check_card_instance) = check_card_instance {
                            let check_slot_pos = location_ids::get_slot_position(check_card_instance.location, &resources.board);
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
            cards.retain(|c| {
                for t in contains_types {
                    return c.card_types.contains(t);
                }
                false
            });
        }

        if let Some(id_is) = &self.id_is {
            cards.retain(|c| {
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
pub enum CardBehaviorAction {
    DrawCard {
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
        card: String,
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
    CreateCard {
        location: LocationTarget,
        id: String,
    }
}

impl CardBehaviorAction {
    pub async fn run(&self, context: &mut GameContext, resources: &mut StateResources, state: &mut StateMachine, communicator: &mut GameCommunicator) -> Result<CardBehaviorResult> {
        let this = context.get(context_keys::ACTION_THIS)?.as_token_instance_id()?;

        let result = match self {
            CardBehaviorAction::DrawCard { target } => {
                for player in target.evaluate(context.get(context_keys::OWNER)?.as_player_id()?) {
                    state.draw_card(player);
                }
                CardBehaviorResult::Ok
            }
            CardBehaviorAction::Replace { target, replacement } => {
                let target = target.evaluate(&context, resources)?;
                for card_instance_id in &target {
                    let card_instance = resources.token_instances.get(card_instance_id).context("Tried to replace a card that does not exist")?;
                    let location = card_instance.location;
                    let owner = card_instance.owner;
                    state.create_token(replacement, owner, location);
                    state.destroy_token(this, *card_instance_id);
                }

                CardBehaviorResult::Ok
            },
            CardBehaviorAction::AddTypes { target, types } => todo!(),
            CardBehaviorAction::ModifyAttack { target, amount } => todo!(),
            CardBehaviorAction::ModifyHealth { target, amount } => todo!(),
            CardBehaviorAction::ModifyDefense { target, amount } => todo!(),
            CardBehaviorAction::ModifyCost { target, amount } => todo!(),
            CardBehaviorAction::Destroy { target } => {
                if let Ok(cards) = target.evaluate(context, resources) {
                    for card in cards {
                        state.destroy_token(this, card);
                    }
                }
                CardBehaviorResult::Ok
            }
            CardBehaviorAction::Summon { target, card } => todo!(),

            CardBehaviorAction::GiveAllTypes { .. } => todo!(),
            CardBehaviorAction::Cancel => CardBehaviorResult::Cancel,
            CardBehaviorAction::SelectUnit { .. } => todo!(),
            CardBehaviorAction::SaveContext { .. } => todo!(),
            CardBehaviorAction::SumAttack { target, filter } => todo!(),
            CardBehaviorAction::AddBehavior { .. } => todo!(),
            CardBehaviorAction::RemoveBehavior { .. } => todo!(),
            CardBehaviorAction::SetCounter { .. } => todo!(),
            CardBehaviorAction::ModifyCounter { .. } => todo!(),
            CardBehaviorAction::CreateCard { .. } => todo!(),
            CardBehaviorAction::DamageHero { target, amount } => {
                for target in target.evaluate(context.get(context_keys::OWNER)?.as_player_id()?) {
                    state.deal_effect_damage(this, resources.get_player(target).hero, *amount as i32);
                }
                CardBehaviorResult::Ok
            },
            CardBehaviorAction::DamageUnit { target, amount } => {
                for card_instance_id in target.evaluate(context, resources)? {
                    state.deal_effect_damage(this, card_instance_id, *amount as i32);
                }
                CardBehaviorResult::Ok
            },
            CardBehaviorAction::RedirectTarget { new_target } => {
                let new_target = new_target.evaluate(context, resources)?;
                let new_target = new_target.first();
                if let Some(new_target) = new_target {
                    todo!();
                }
                CardBehaviorResult::Ok
            }
        };

        Ok(result)
    }
}