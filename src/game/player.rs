use std::collections::VecDeque;
use color_eyre::Result;
use crate::game::cards::card::{CardInstance, CardRegistry};
use crate::game::instruction::{Instruction, InstructionQueue};
use crate::game::location::{Location, LocationInstance};

pub type PlayerInstance<'a> = &'a Player<'a>;

pub struct Player<'a> {
    pub thaum: u32,
    pub id: u32,
    pub deck: LocationInstance<'a>,
    pub hand: LocationInstance<'a>,
}

impl<'a : 'c, 'c : 'b, 'b> Player<'a> {
    pub fn new(id: u32, deck: LocationInstance<'a>, hand: LocationInstance<'a>) -> Self {
        Self {
            id,
            thaum: 0,
            deck,
            hand,
        }
    }
    
    pub fn set_thaum(&'a self, thaum: u32, queue: &'b mut VecDeque<Instruction<'b>>) {
        queue.push_back(Instruction::SetThaum(self, thaum));
    }

    pub fn populate_deck(&self, data: &str, registry: &'c CardRegistry, queue: InstructionQueue) -> Result<()> {
        let splits = data.split(',');
        for split in splits {
            let parsed_id = split.parse::<u32>()?;
            let card = registry.create_card(parsed_id, 0, self);
            match card {
                Ok(c) => queue.push_back(Instruction::CreateCard(c, self.deck, self)),
                Err(_) => {}
            }
        }
        Ok(())
    }
}

pub struct CardCollection<'a> {
    pub cards : Vec<CardInstance<'a>>,
    pub lid : u32,
}

impl<'a> Location for CardCollection<'a> {
    fn set_lid(&mut self, lid: u32) {
        self.lid = lid;
    }

    fn get_lid(&self) -> u32 {
        self.lid
    }
}

impl<'a> CardCollection<'a> {
    pub fn new() -> Self {
        Self {
            cards: vec!(),
            lid: 0,
        }
    }
    
    pub fn shuffle(&self) {
        todo!()
    }
    
    pub fn draw(&self) -> CardInstance {
        todo!()
    }
}