use crate::game::cards::card::CardInstance;

pub type LocationInstance<'a> = &'a Box<dyn Location>;

pub trait Location {
    fn set_lid(&mut self, lid: u32);
    fn get_lid(&self) -> u32;

    fn add_card(&mut self, card: CardInstance) {}
    fn remove_card(&mut self, card: CardInstance) {}
}