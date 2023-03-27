use std::collections::HashMap;
use color_eyre::eyre::{ContextCompat, eyre};

use crate::game::board::Board;
use crate::game::cards::token_deserializer::TokenData;
use crate::game::id_types::{TokenInstanceId, PlayerId, LocationId};
use crate::game::player::Player;
use crate::game::state_resources::StateResources;

use color_eyre::Result;

pub mod context_keys {
    pub const OWNER: &str = "owner";
    pub const TOKEN_INSTANCE: &str = "token_instance";
    pub const EQUIP_TARGET: &str = "equip_target";
    pub const EQUIPPING_ITEM: &str = "equipping_item";
    pub const TRIGGER_THIS: &str = "trigger_this";
    pub const ACTION_THIS: &str = "action_this";
    pub const ATTACKER: &str = "attacker";
    pub const DEFENDER: &str = "defender";
    pub const FROM_LOCATION: &str = "from_location";
    pub const TO_LOCATION: &str = "to_location";
    pub const CANCEL: &str = "cancel";
    pub const SELECTED_TOKEN: &str = "selected_token";
    pub const IS_COUNTER_ATTACK: &str = "is_counter_attack";
    pub const EFFECT_DAMAGE: &str = "effect_damage";
}

#[derive(Clone, PartialEq, Debug)]
pub struct GameContext {
    pub values: HashMap<String, ContextValue>,
}

impl GameContext {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: &str, value: ContextValue) {
        self.values.insert(key.into(), value);
    }
    
    pub fn get(&self, key: &str) -> Result<&ContextValue> {
        self.values.get(key).context(format!("Failed to find key {key} in GameContext"))
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut ContextValue> {
        self.values.get_mut(key)
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }

    pub fn append(&mut self, other: &GameContext) {
        for (key, value) in &other.values {
            self.insert(&key, value.clone());
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum ContextValue {
    String(String),
    U64(u64),
    I64(i64),
    F64(f64),
    Bool(bool),
    TokenInstanceId(TokenInstanceId),
    LocationId(LocationId),
    PlayerId(PlayerId),
    Array(Vec<ContextValue>),
}

impl ContextValue {
    pub fn as_string(&self) -> Result<&String> {
        match self {
            ContextValue::String(string) => Ok(string),
            _ => Err(eyre!("Tried to cast context value to String when it is not a String")),
        }
    }
    
    pub fn as_u64(&self) -> Result<u64> {
        match self {
            ContextValue::U64(uint) => Ok(*uint),
            _ => Err(eyre!("Tried to cast context value to u64 when it is not a u64")),
        }
    }
    
    pub fn as_i64(&self) -> Result<i64> {
        match self {
            ContextValue::I64(int) => Ok(*int),
            _ => Err(eyre!("Tried to cast context value to i64 when it is not a i64")),
        }
    }
    
    pub fn as_array(&self) -> Result<&Vec<ContextValue>> {
        match self {
            ContextValue::Array(array) => Ok(array),
            _ => Err(eyre!("Tried to cast context value to Array when it is not a Array")),
        }
    }
    
    pub fn as_bool(&self) -> Result<bool> {
        match self {
            ContextValue::Bool(bool) => Ok(*bool),
            _ => Err(eyre!("Tried to cast context value to bool when it is not a bool")),
        }
    }
    
    pub fn as_f64(&self) -> Result<f64> {
        match self {
            ContextValue::F64(float) => Ok(*float),
            _ => Err(eyre!("Tried to cast context value to f64 when it is not a f64")),
        }
    }

    pub fn as_token_instance_id(&self) -> Result<TokenInstanceId> {
        match self {
            ContextValue::TokenInstanceId(token) => Ok(*token),
            _ => Err(eyre!("Tried to cast context value to TokenInstanceId when it is not a TokenInstanceId")),
        }
    }

    pub fn as_location_id(&self) -> Result<LocationId> {
        match self {
            ContextValue::LocationId(location) => Ok(*location),
            _ => Err(eyre!("Tried to cast context value to LocationId when it is not a LocationId")),
        }
    }

    pub fn as_player_id(&self) -> Result<PlayerId> {
        match self {
            ContextValue::PlayerId(player) => Ok(*player),
            _ => Err(eyre!("Tried to cast context value to PlayerId when it is not a PlayerId")),
        }
    }
}