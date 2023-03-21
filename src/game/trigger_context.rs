﻿use std::collections::HashMap;

use crate::game::board::Board;
use crate::game::cards::card_deserialization::Card;
use crate::game::id_types::{CardInstanceId, PlayerId};
use crate::game::player::Player;
use crate::game::state_resources::StateResources;

#[derive(Clone)]
pub struct CardBehaviorContext {
    pub owner: PlayerId,
    values: HashMap<String, ContextValue>,
}

impl CardBehaviorContext {
    pub fn new(owner: PlayerId) -> Self {
        Self {
            values: HashMap::new(),
            owner,
        }
    }
    
    pub fn insert(&mut self, key: &str, value: ContextValue) {
        self.values.insert(key.into(), value);
    }
    
    pub fn get(&self, key: &str) -> Option<&ContextValue> {
        self.values.get(key)
    }
    
    pub fn contains_key(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }
}

#[derive(Clone)]
pub enum ContextValue {
    String(String),
    U64(u64),
    I64(i64),
    F64(f64),
    Bool(bool),
    CardInstance(CardInstanceId),
    Array(Vec<ContextValue>),
}

impl ContextValue {
    pub fn as_string(&self) -> Option<&String> {
        match self {
            ContextValue::String(s) => Some(s),
            _ => None,
        }
    }
    
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            ContextValue::U64(u) => Some(*u),
            _ => None,
        }
    }
    
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            ContextValue::I64(i) => Some(*i),
            _ => None,
        }
    }
    
    pub fn as_array(&self) -> Option<&Vec<ContextValue>> {
        match self {
            ContextValue::Array(a) => Some(a),
            _ => None,
        }
    }
    
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ContextValue::Bool(b) => Some(*b),
            _ => None,
        }
    }
    
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            ContextValue::F64(f) => Some(*f),
            _ => None,
        }
    }

    pub fn as_card_instance(&self) -> Option<CardInstanceId> {
        match self {
            ContextValue::CardInstance(c) => Some(*c),
            _ => None,
        }
    }
}