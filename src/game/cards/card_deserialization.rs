use std::fmt;
use serde::de::{Error, MapAccess, Unexpected, Visitor};
use serde::{de, Deserialize, Deserializer};
use serde::__private::de::EnumDeserializer;
use serde::de::value::StringDeserializer;
use serde_enum_str::Deserialize_enum_str;

#[derive(Deserialize, Debug)]
pub struct Card {
    #[serde(skip_deserializing)]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
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
        attack: u32,
        health: u32,
        defense: u32,
    },
    Item,
    Command,
}

#[derive(Deserialize, Debug, Clone)]
pub struct CardBehavior {
    pub description: String,

    #[serde(rename = "trigger")]
    pub triggers: Vec<CardBehaviorTrigger>,

    #[serde(rename = "action")]
    pub actions: Vec<CardBehaviorAction>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct CardBehaviorTrigger {
    pub when: CardBehaviorTriggerWhen
}

#[derive(Debug, Clone)]
pub struct CardBehaviorTriggerWhen {
    pub activator: CardBehaviorTriggerWhenActivator,
    pub name: CardBehaviorTriggerWhenName,
}

#[derive(Debug, Deserialize_enum_str, Clone)]
#[serde(rename_all = "snake_case")]
pub enum CardBehaviorTriggerWhenActivator {
    #[serde(alias = "Owner")]
    Owned,

    Opponent,
    This,
    Either
}

#[derive(Debug, Deserialize_enum_str, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CardBehaviorTriggerWhenName {
    WillBeSummoned,
    HasBeenSummoned,
    WillCast,
    HasCast,
    WillAttack,
    WillBeAttacked,
    HasAttacked,
    HasBeenAttacked,
    TookDamage,
    HasDestroyed,
    HasBeenDestroyed,
    HasDefeated,
    HasBeenDefeated,
    WasDrawn,
    DrawCard,
    TurnEnded,
    TurnStarted,
    WillEnterLandscape,
    HasEnteredLandscape
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
#[serde(rename_all = "snake_case")]
pub enum CardBehaviorActionPlayerTarget {
    Owner,
    Opponent,
    Random,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum CardBehaviorActionUnitTarget {
    This,
    Find, // Todo: Should use the same syntax as the trigger's "and" field
    FindMany,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum CardBehaviorActionCardTarget {
    This,
    Find,
    FindMany,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case", tag = "then", content = "with")]
pub enum CardBehaviorAction {
    DrawCard {
        target: CardBehaviorActionPlayerTarget,
    },
    Replace {
        target: CardBehaviorActionUnitTarget,
        replacement: String,
    },
    AddTypes {
        target: CardBehaviorActionUnitTarget,
        types: Vec<String>,
    },
    ModifyAttack {
        target: CardBehaviorActionUnitTarget,
        amount: i32,
    },
    ModifyHealth {
        target: CardBehaviorActionUnitTarget,
        amount: i32,
    },
    ModifyDefense {
        target: CardBehaviorActionUnitTarget,
        amount: i32,
    },
    ModifyCost {
        target: CardBehaviorActionUnitTarget,
        amount: i32,
    },
    Destroy {
        target: CardBehaviorActionCardTarget,
    },
    Summon {
        target: CardBehaviorActionUnitTarget,
        card: String,
    },
}