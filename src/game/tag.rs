use color_eyre::eyre::ContextCompat;
use color_eyre::Result;
use crate::game::cards::card::CardInstance;
use crate::game::location::{LocationInstance};
use crate::game::player::PlayerInstance;

pub enum Tag<'a> {
    Player(PlayerInstance<'a>),
    Thaum(u32),
    Card(u32),
    CardInstance(CardInstance<'a>),
    From(LocationInstance<'a>),
    To(LocationInstance<'a>),
}

impl<'a> Tag<'a> {
    pub fn build(self) -> String {
        match self {
            Tag::Player(p) => format!("/player/{}/!player/", p.id),
            Tag::Thaum(t) => format!("/thaum/{}/!thaum/", t),
            Tag::Card(c) => format!("/card/{}/!card/", c),
            Tag::CardInstance(c) => format!("/iid/{}/!iid/", c.iid),
            Tag::From(l) => format!("/from/{}/!from/", l.get_lid()),
            Tag::To(l) => format!("/to/{}/!to/", l.get_lid()),
        }
    }
}

pub fn get_tag(tag: &str, data: &str) -> Result<String> {
    let start = data.find(&format!("/{tag}/")).context("Tried to get tag in data but no tag was found")?;
    let end = data.find(&format!("/!{tag}/")).context("Tried to get tag in data but no closing tag was found")?;
    return Ok((&data[start + tag.len() + 2..end]).into());
}