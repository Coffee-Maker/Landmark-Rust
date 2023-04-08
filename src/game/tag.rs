use color_eyre::eyre::ContextCompat;
use color_eyre::Result;
use crate::game::tokens::token_deserializer::{TokenData, TokenCategory};

use crate::game::tokens::token_instance::TokenInstance;
use crate::game::prompts::PromptType;
use crate::game::id_types::{TokenInstanceId, LocationId, PlayerId, PromptInstanceId, ServerInstanceId};

pub enum Tag {
    Player(PlayerId),
    U64(u64),
    F32(f32),
    String(String),
    TokenInstanceData(TokenInstance),
    TokenData(TokenData),
    TokenBehaviors(TokenInstance),
    ServerInstanceId(ServerInstanceId),
    TokenInstanceId(TokenInstanceId),
    LocationId(LocationId),
    PromptInstanceId(PromptInstanceId),
    PromptType(PromptType),
}

impl Tag {
    pub fn build(self) -> Result<String> {
        Ok(format!("//{}/!", (match self {
            Tag::Player(p) => format!("{}", p as u32),
            Tag::U64(t) => format!("{}", t),
            Tag::F32(t) => format!("{}", t),
            Tag::String(c) => format!("{}", c),
            Tag::TokenInstanceData(c) => {
                let id = c.token_data.id.clone();
                let name = format!("{} ({})", c.token_data.name.clone(), c.token_data.cost);
                let description = c.token_data.description.clone().unwrap_or("".to_string()); // Todo: Is this the correct method for a default?
                let cost = c.cost;
                let mut health = c.current_stats.health;
                let mut attack = c.current_stats.attack;
                let mut defense = c.current_stats.defense;
                let types = c.token_types.join(", ");
                let token_category = match &c.token_data.token_category {
                    TokenCategory::Hero { .. } => 0,
                    TokenCategory::Landscape { .. } => 1,
                    TokenCategory::Unit { .. } => 2,
                    TokenCategory::Item => 3,
                    TokenCategory::Command => 4,
                };
                format!("{id};;{token_category};;{name};;{description};;{cost};;{health};;{defense};;{attack};;{types};;")
            },
            Tag::TokenData(c) => {
                let id = c.id.clone();
                let name = format!("{} ({})", c.name, c.cost);
                let description = c.description.clone().unwrap_or(" ".to_string()); // Todo: Is this the correct method for a default?
                let cost = c.cost;
                let mut health = 0;
                let mut attack = 0;
                let mut defense = 0;
                let types = c.types.join(", ");
                let token_category = match c.token_category {
                    TokenCategory::Hero { health: h , defense: d } =>  {
                        health = h;
                        defense = d;
                        0
                    },
                    TokenCategory::Landscape { .. } => 1,
                    TokenCategory::Unit { health: h, defense: d, attack: a } => {
                        health = h;
                        defense = d;
                        attack = a;
                        2
                    },
                    TokenCategory::Item => 3,
                    TokenCategory::Command => 4,
                };
                format!("{id};;{token_category};;{name};;{description};;{cost};;{health};;{defense};;{attack};;{types};;")
            },
            Tag::TokenBehaviors(c) => {
                let mut string_to_send = String::new();
                for behavior in c.behaviors {
                    if let Some(name) = behavior.name {
                        string_to_send = format!("{}{};;{};;", string_to_send, name, behavior.description.unwrap_or("".to_string()));
                    }
                }
                string_to_send
            },
            Tag::ServerInstanceId(c) => format!("{}", c),
            Tag::TokenInstanceId(c) => format!("{}", c),
            Tag::LocationId(c) => format!("{}", c),
            Tag::PromptInstanceId(id) => format!("{}", id),
            Tag::PromptType(t) => format!("{:?}", t),
        })))
    }
}

pub fn get_tag(tag: &str, data: &str) -> Result<String> {
    let start = data
        .find(&format!("/{tag}/"))
        .context("Tried to get tag in data but no tag was found")?;
    let end = data
        .find(&format!("/!{tag}/"))
        .context("Tried to get tag in data but no closing tag was found")?;
    return Ok((&data[start + tag.len() + 2..end]).into());
}
