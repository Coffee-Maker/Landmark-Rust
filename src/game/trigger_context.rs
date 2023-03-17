use std::collections::HashMap;
use toml::{Table, Value};
use crate::game::cards::card::CardData;
use crate::game::game_state::{CardKey, GameState};

pub struct TriggerContext {
    values: HashMap<String, ContextValue>,
}

impl TriggerContext {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }
    
    pub fn add_card(&mut self, state: &GameState, card_key: CardKey) {
        let card = state.card_instances.get(card_key).unwrap();
        self.values.insert("iid".into(), ContextValue::U64(card.instance_id));
        self.values.insert("id".into(), ContextValue::String(card.card_id.clone()));
        self.values.insert("type".into(), ContextValue::Array(card.card_types.iter().map(|s| ContextValue::String(s.clone())).collect()));
        self.values.insert("thaum".into(), ContextValue::I64(card.cost as i64));
        self.values.insert("name".into(), ContextValue::String(card.name.clone()));
        self.values.insert("description".into(), ContextValue::String(card.description.clone()));

        match state.board.get_relevant_landscape(state, card.key) {
            Some(lid) => {
                let landscape = state.locations.get(lid).unwrap();
                let landscape_card = landscape.get_card().unwrap();
                let landscape_types = &state.card_instances.get(landscape_card).unwrap().card_types;
                self.insert("landscape_type", ContextValue::Array(landscape_types.iter().map(|c| ContextValue::String(c.to_string())).collect()));
            }
            None => {}
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

pub enum ContextValue {
    String(String),
    U64(u64),
    I64(i64),
    F64(f64),
    Bool(bool),
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
}