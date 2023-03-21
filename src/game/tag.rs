use color_eyre::eyre::ContextCompat;
use color_eyre::Result;
use crate::game::cards::card_deserialization::CardCategory;

use crate::game::cards::card_instance::CardInstance;
use crate::game::id_types::{CardInstanceId, LocationId, PlayerId, ServerInstanceId};

pub enum Tag {
    Player(PlayerId),
    U64(u64),
    String(String),
    CardData(CardInstance),
    ServerInstanceId(ServerInstanceId),
    CardInstanceId(CardInstanceId),
    LocationId(LocationId),
}

impl Tag {
    pub fn build(self) -> Result<String> {
        Ok(format!("//{}/!", (match self {
            Tag::Player(p) => format!("{}", p as u32),
            Tag::U64(t) => format!("{}", t),
            Tag::String(c) => format!("{}", c),
            Tag::CardData(c) => {
                let id = c.card.id.clone();
                let name = c.card.name.clone();
                let description = c.card.description.clone().unwrap_or("".to_string()); // Todo: Is this the correct method for a default?
                let cost = c.cost;
                let mut health = 0;
                let mut attack = 0;
                let mut defense = 0;
                let types = c.card_types.join(", ");
                let card_category = match &c.card.card_category {
                    CardCategory::Hero => 0,
                    CardCategory::Landscape { slots: _slots } => 1,
                    CardCategory::Unit { attack: a, health: h, defense: d } => {
                        attack = *a;
                        health = *h;
                        defense = *d;
                        2
                    }
                    CardCategory::Item => 3,
                    CardCategory::Command => 4,
                };
                format!("{id};;{card_category};;{name};;{description};;{cost};;{health};;{defense};;{attack};;{types};;")
            }
            Tag::ServerInstanceId(c) => format!("{}", c),
            Tag::CardInstanceId(c) => format!("{}", c),
            Tag::LocationId(c) => format!("{}", c),
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
