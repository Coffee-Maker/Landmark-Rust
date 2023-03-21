use std::fmt;
use color_eyre::eyre::ContextCompat;

use color_eyre::Result;
use serde::{de, Deserialize, Deserializer};
use serde::__private::de::EnumDeserializer;
use serde::de::{Error, MapAccess, Unexpected, Visitor};
use serde::de::value::StringDeserializer;
use serde_enum_str::Deserialize_enum_str;

use crate::game::cards::card_behaviors::CardBehaviorResult;
use crate::game::game_communicator::GameCommunicator;
use crate::game::game_state::{CardBehaviorTriggerQueue, CardBehaviorTriggerWithContext, GameState};
use crate::game::id_types::{CardInstanceId, PlayerId};
use crate::game::id_types::PlayerId::{Player1, Player2};
use crate::game::trigger_context::CardBehaviorContext;

#[derive(Deserialize, Debug)]
pub struct Card {
    #[serde(skip_deserializing)] pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub cost: u64,
    pub types: Vec<String>,

    #[serde(flatten)]
    pub card_category: CardCategory,

    #[serde(rename = "behavior", default)]
    pub behaviors: Vec<CardBehavior>,
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct SlotPosition {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case", tag = "category")]
pub enum CardCategory {
    Hero,
    Landscape {
        slots: Vec<SlotPosition>
    },
    Unit {
        health: u32,
        #[serde(default)] attack: u32,
        #[serde(default)] defense: u32,
    },
    Item,
    Command,
}

#[derive(Deserialize, Debug, Clone)]
pub struct CardBehavior {
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
    TurnStarted,
    TurnEnded,

    // Card
    WillDrawCard,
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
        target: CardTarget,
        types: Vec<String>,
    }
}

impl CardBehaviorTriggerAnd {
    pub async fn check(&self, context: &CardBehaviorContext, state: &mut GameState, communicator: &mut GameCommunicator) -> Result<bool> {
        Ok(match self {
            CardBehaviorTriggerAnd::TypeContains { target, types } => {
                let target = target.evaluate(context)?;
                let target_instance = state.resources.card_instances.get(&target).context(format!("Card with the id {target} was not a found in state resources"))?;
                let mut passed = true;
                for t in types {
                    if target_instance.card_types.contains(t) == false {
                        passed = false;
                        break;
                    }
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
    pub fn get(&self, owner: PlayerId) -> Vec<PlayerId> {
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
    Find, // Todo: Should use the same syntax as the trigger's "and" field
    All,
}

impl Default for UnitTarget {
    fn default() -> Self {
        Self::All
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum CardTarget {
    This,
    EquipTarget,
    Find {
        filter: CardFilter
    },
    Context {
        context_key: String
    }
}

impl CardTarget {
    pub fn evaluate(&self, context: &CardBehaviorContext) -> Result<CardInstanceId> {
        Ok(match self {
            CardTarget::This => todo!(),
            CardTarget::EquipTarget => todo!(),
            CardTarget::Find { .. } => todo!(),
            CardTarget::Context { context_key } => context.get(context_key).context(format!("Context does not contain the key {context_key}"))?.as_card_instance().context(format!("Context value with key {context_key} was not a card instance"))?,
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
pub struct CardFilter {
    owned_by: Option<PlayerTarget>,
    adjacent_to: Option<UnitTarget>,
    contains_types: Option<Vec<String>>,
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
        target: CardTarget,
    },
    Summon {
        target: UnitTarget,
        card: String,
    },

    // New
    GiveAllTypes {
        target: CardTarget
    },
    Cancel,
    SelectUnit {
        context_key: String,
        filter: CardFilter,
    },
    SaveContext {
        context_key: String,
        personal_key: String,
    },
    SumAttack {
        target: UnitTarget,
        filter: CardFilter,
    },
    AddBehavior {
        target: CardTarget,
        behavior: String,
    },
    RemoveBehavior {
        target: CardTarget,
        behavior: String,
    },
    SetCounter {
        target: CardTarget,
        counter: String,
        value: i32,
    },
    ModifyCounter {
        target: CardTarget,
        counter: String,
        amount: i32,
    },
    CreateCard {
        location: LocationTarget,
        id: String,
    }
}

impl CardBehaviorAction {
    pub async fn run(&self, context: &CardBehaviorContext, state: &mut GameState, communicator: &mut GameCommunicator) -> Result<(CardBehaviorTriggerQueue, CardBehaviorResult)> {
        let queue = CardBehaviorTriggerQueue::new();

        let result = match self {
            CardBehaviorAction::DrawCard { target } => {
                for player in target.get(context.owner) {
                    let player = match player {
                        PlayerId::Player1 => &mut state.player_1,
                        PlayerId::Player2 => &mut state.player_2,
                    };
                    player.draw_card(&mut state.resources, communicator).await?;
                }
                CardBehaviorResult::Ok
            }
            CardBehaviorAction::Replace { target, replacement } => todo!(),
            CardBehaviorAction::AddTypes { target, types } => todo!(),
            CardBehaviorAction::ModifyAttack { target, amount } => todo!(),
            CardBehaviorAction::ModifyHealth { target, amount } => todo!(),
            CardBehaviorAction::ModifyDefense { target, amount } => todo!(),
            CardBehaviorAction::ModifyCost { target, amount } => todo!(),
            CardBehaviorAction::Destroy { target } => todo!(),
            CardBehaviorAction::Summon { target, card } => todo!(),

            CardBehaviorAction::GiveAllTypes { .. } => todo!(),
            CardBehaviorAction::Cancel => CardBehaviorResult::Cancel,
            CardBehaviorAction::SelectUnit { .. } => todo!(),
            CardBehaviorAction::SaveContext { .. } => todo!(),
            CardBehaviorAction::SumAttack { .. } => todo!(),
            CardBehaviorAction::AddBehavior { .. } => todo!(),
            CardBehaviorAction::RemoveBehavior { .. } => todo!(),
            CardBehaviorAction::SetCounter { .. } => todo!(),
            CardBehaviorAction::ModifyCounter { .. } => todo!(),
            CardBehaviorAction::CreateCard { .. } => todo!(),
        };

        Ok((queue, result))
    }
}