use std::fmt;
use serde::de::{Error, MapAccess, Unexpected, Visitor};
use serde::{de, Deserialize, Deserializer};
use serde::__private::de::EnumDeserializer;
use serde::de::value::StringDeserializer;

#[derive(Deserialize, Debug)]
pub struct Card {
    name: String,
    description: String,
    cost: u64,
    types: Vec<String>,

    #[serde(flatten)]
    card_category: CardCategory,

    #[serde(rename = "behavior")]
    behaviors: Vec<CardBehavior>,
}

#[derive(Deserialize, Debug)]
pub struct SlotPosition {
    x: i32,
    y: i32,
    z: i32,
}

#[derive(Deserialize, Debug)]
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

#[derive(Deserialize, Debug)]
pub struct CardBehavior {
    description: String,

    #[serde(rename = "trigger")]
    triggers: Vec<CardBehaviorTrigger>,

    #[serde(rename = "action")]
    actions: Vec<CardBehaviorAction>,
}

#[derive(Deserialize, Debug)]
pub struct CardBehaviorTrigger {
    when: CardBehaviorTriggerWhen
}

#[derive(Debug)]
pub struct CardBehaviorTriggerWhen {
    activator: CardBehaviorTriggerWhenActivator,
    name: CardBehaviorTriggerWhenName,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum CardBehaviorTriggerWhenActivator {
    #[serde(alias = "Owner")]
    Owned,

    Opponent,
    This,
    Either
}

#[derive(Deserialize, Debug)]
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
}

impl<'de> Deserialize<'de> for CardBehaviorTriggerWhen {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct CardBehaviorTriggerWhenVisitor;

        impl<'de> Visitor<'de> for CardBehaviorTriggerWhenVisitor {
            type Value = (String, String);

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("\"<target>:<trigger>\"")
            }

            fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
                let [activator, name] = v.split(':').collect::<Vec<_>>()[..] else {
                    return Err(de::Error::invalid_value(Unexpected::Str(v), &self))
                };

                // Ok(CardBehaviorTriggerWhen {
                //     activator: serde::de::toml::from_str::<CardBehaviorTriggerWhenActivator>(activator.into())
                //         .map_err(|_| de::Error::custom("Trigger activator is not a valid variant."))?,
                //     name: toml::from_str::<CardBehaviorTriggerWhenName>(name.into())
                //         .map_err(|_| de::Error::custom("Trigger name is not a valid variant."))?,
                // })

                Ok((activator.into(), name.into()))
            }
        }

        let (activator, name) = deserializer.deserialize_string(CardBehaviorTriggerWhenVisitor)?;

        EnumDeserializer::new(activator).deserialize_enum()

        Ok(CardBehaviorTriggerWhen {
            activator: CardBehaviorTriggerWhenActivator::deserialize(deserializer)?,
            name: CardBehaviorTriggerWhenName::HasAttacked,
        })
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case", tag = "then", content = "with")]
pub enum CardBehaviorAction {
    RetireSelectedUnits {
        amount: u64
    }
}