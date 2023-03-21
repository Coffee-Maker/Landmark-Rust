use std::fmt;
use serde::de::{Error, MapAccess, Unexpected, Visitor};
use serde::{de, Deserialize, Deserializer};
use serde::__private::de::EnumDeserializer;
use serde::de::value::StringDeserializer;
use serde_enum_str::Deserialize_enum_str;

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
    WillDrawCard,
    HasBeenDrawn,
    WillDestroy,
    WillBeDestroyed,
    HasDestroyed,
    HasBeenDestroyed,

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
    #[serde(default)] owned_by: PlayerTarget,
    #[serde(default)] adjacent_to: UnitTarget,
    #[serde(default)] contains_types: Vec<String>,
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