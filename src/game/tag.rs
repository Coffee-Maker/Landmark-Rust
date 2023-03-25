use color_eyre::eyre::ContextCompat;
use color_eyre::Result;
use crate::game::cards::card_deserialization::{Card, CardCategory};

use crate::game::cards::card_instance::CardInstance;
use crate::game::prompts::PromptType;
use crate::game::id_types::{CardInstanceId, LocationId, PlayerId, PromptInstanceId, ServerInstanceId};

pub enum Tag {
    Player(PlayerId),
    U64(u64),
    F32(f32),
    String(String),
    CardInstanceData(CardInstance),
    CardData(Card),
    CardBehaviors(CardInstance),
    ServerInstanceId(ServerInstanceId),
    CardInstanceId(CardInstanceId),
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
            Tag::CardInstanceData(c) => {
                let id = c.card.id.clone();
                let name = format!("{} ({})", c.card.name.clone(), c.card.cost);
                let description = c.card.description.clone().unwrap_or("".to_string()); // Todo: Is this the correct method for a default?
                let cost = c.cost;
                let mut health = c.current_stats.health;
                let mut attack = c.current_stats.attack;
                let mut defense = c.current_stats.defense;
                let types = c.card_types.join(", ");
                let card_category = match &c.card.card_category {
                    CardCategory::Hero { .. } => 0,
                    CardCategory::Landscape { .. } => 1,
                    CardCategory::Unit { .. } => 2,
                    CardCategory::Item => 3,
                    CardCategory::Command => 4,
                };
                format!("{id};;{card_category};;{name};;{description};;{cost};;{health};;{defense};;{attack};;{types};;")
            },
            Tag::CardData(c) => {
                let id = c.id.clone();
                let name = format!("{} ({})", c.name, c.cost);
                let description = c.description.clone().unwrap_or(" ".to_string()); // Todo: Is this the correct method for a default?
                let cost = c.cost;
                let mut health = 0;
                let mut attack = 0;
                let mut defense = 0;
                let types = c.types.join(", ");
                let card_category = match c.card_category {
                    CardCategory::Hero { health: h , defense: d } =>  {
                        health = h;
                        defense = d;
                        0
                    },
                    CardCategory::Landscape { .. } => 1,
                    CardCategory::Unit { health: h, defense: d, attack: a } => {
                        health = h;
                        defense = d;
                        attack = a;
                        2
                    },
                    CardCategory::Item => 3,
                    CardCategory::Command => 4,
                };
                format!("{id};;{card_category};;{name};;{description};;{cost};;{health};;{defense};;{attack};;{types};;")
            },
            Tag::CardBehaviors(c) => {
                let mut string_to_send = String::new();
                for behavior in c.behaviors {
                    if let Some(name) = behavior.name {
                        string_to_send = format!("{}{};;{};;", string_to_send, name, behavior.description.unwrap_or("".to_string()));
                    }
                }
                string_to_send
            },
            Tag::ServerInstanceId(c) => format!("{}", c),
            Tag::CardInstanceId(c) => format!("{}", c),
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
