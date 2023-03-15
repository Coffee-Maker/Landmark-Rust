use std::collections::VecDeque;
use color_eyre::eyre::ContextCompat;
use crate::game::cards::card::{CardBehaviour, CardData, CardInstance, MutCardInstance};
use crate::game::game_communicator::GameCommunicator;
use crate::game::location::LocationInstance;
use crate::game::player::{Player, PlayerInstance};
use crate::game::tag::Tag;

use color_eyre::Result;
use crate::game::game_state::GameState;

pub type InstructionQueue<'a> = &'a mut VecDeque<Instruction<'a>>;

pub enum Instruction<'a> {
    SetThaum(PlayerInstance<'a>, u32),
    MoveCard(CardInstance<'a>, LocationInstance<'a>, LocationInstance<'a>),
    CreateCard(CardData<'a>, LocationInstance<'a>, PlayerInstance<'a>),
}

impl<'a> Instruction<'a> {
    fn process(self, state: &mut GameState<'a>, comm: &mut GameCommunicator) -> Result<()> {
        match self {
            Instruction::SetThaum(p, t) => {
                let id = p.id;
                if id == 0 {
                    state.player1.as_mut().unwrap().thaum = t;
                } else if id == 1 {
                    state.player2.as_mut().unwrap().thaum = t;
                }
                comm.send_instruction(&self)
            }
            Instruction::MoveCard(c, f, t) => {
                let mut from = state.locations.get_mut(&f.get_lid()).context("Tried to move a card from a location that doesn't exist")?;
                let mut to = state.locations.get_mut(&t.get_lid()).context("Tried to move a card to a location that doesn't exist")?;
                let mut card = state.card_instances.get_mut(&c.id).context("Tried to move a card that doesn't exist")?;
                to.remove_card(card);
                to.add_card(card);
                comm.send_instruction(&self)
            }
            Instruction::CreateCard(c, l, o) => {
                let mut loc = state.locations.get_mut(&l.get_lid()).context("Tried to create a card to a location that does not exist")?;
                let c = &state.card_instances.insert(0, c).unwrap();
                loc.add_card(c);
                comm.send_instruction(&self)
            }
        }
    }

    pub fn build(&self) -> String {
        match self {
            Instruction::SetThaum(p, t) => format!("sth{}{}", Tag::Player(p).build(), Tag::Thaum(*t).build()),
            Instruction::MoveCard(c, f, t) => format!("mve{}{}{}", Tag::CardInstance(c).build(), Tag::From(f).build(), Tag::To(t).build()),
            Instruction::CreateCard(c, l, p) => format!("crt{}{}{}{}", Tag::Card(c.id).build(), Tag::CardInstance(&c).build(), Tag::To(l).build(), Tag::Player(p).build())
        }
    }
}