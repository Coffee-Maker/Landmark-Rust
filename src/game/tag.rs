use crate::game::game_state::{CardKey, GameState, LocationKey, PlayerId, ServerInstanceId};
use color_eyre::eyre::ContextCompat;
use color_eyre::Result;
use crate::game::cards::card::{CardData, CardCategory};

pub enum Tag {
    Player(PlayerId),
    Integer(u32),
    String(String),
    CardData(CardData),
    ServerInstanceId(ServerInstanceId),
    CardInstance(CardKey),
    Location(LocationKey),
}

impl Tag {
    pub fn build(self, state: &mut GameState) -> Result<String> {
        Ok(format!("//{}/!", (match self {
            Tag::Player(p) => format!("{}", p as u32),
            Tag::Integer(t) => format!("{}", t),
            Tag::String(c) => format!("{}", c),
            Tag::ServerInstanceId(c) => format!("{}", c),
            Tag::CardInstance(c) => format!("{}", state.card_instances.get(c).context("Tried to create tag for non existent card")?.instance_id),
            Tag::Location(l) => format!("{}", state.locations.get(l).context("Tried to create tag for non existent location")?.get_lid()),
            Tag::CardData(c) => {
                let id = c.card_id.clone();
                let name = c.name.clone();
                let description = c.description.clone();
                let cost = c.cost;
                let mut health = 0;
                let mut attack = 0;
                let mut defense = 0;
                let types = c.card_types.join(", ");
                let card_type = match c.card_category {
                    CardCategory::Hero => 0,
                    CardCategory::Landscape { slots: _slots } => 1,
                    CardCategory::Unit { attack: a, health: h, defense: d } => {
                        attack = a;
                        health = h;
                        defense = d;
                        2
                    }
                    CardCategory::Item => 3,
                    CardCategory::Command => 4,
                };
                format!("{id};;{card_type};;{name};;{description};;{cost};;{health};;{defense};;{attack};;{types};;")
            }
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
