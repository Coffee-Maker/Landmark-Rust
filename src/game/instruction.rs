use crate::game::game_communicator::GameCommunicator;
use crate::game::tag::Tag;
use color_eyre::eyre::ContextCompat;
use std::collections::VecDeque;
use async_recursion::async_recursion;

use crate::game::game_state::{CardKey, GameState, LocationKey, PlayerId, ServerInstanceId};
use color_eyre::Result;
use crate::game::card_slot::CardSlot;
use crate::game::cards::card_behavior::BehaviorTrigger;
use crate::game::trigger_context::TriggerContext;

pub struct InstructionQueue(VecDeque<Instruction>);

impl InstructionQueue {
    pub fn new() -> Self {
        InstructionQueue(VecDeque::new())
    }

    pub fn enqueue(&mut self, instruction: Instruction) {
        self.0.push_back(instruction)
    }

    pub fn dequeue(&mut self) -> Option<Instruction> {
        self.0.pop_front()
    }

    pub fn len(&mut self) -> usize {
        self.0.len()
    }
}

#[derive(Clone)]
pub enum Instruction {
    StartGame {},
    AddLandscapeSlot {
        player: PlayerId,
        index: u32,
        location_id: ServerInstanceId,
    },
    SetThaum {
        player: PlayerId,
        amount: u32,
    },
    MoveCard {
        card: CardKey,
        to: LocationKey,
    },
    DrawCard { player: PlayerId },
    CreateCard {
        id: String,
        iid: ServerInstanceId,
        location: LocationKey,
        player: PlayerId,
    },
    SetTurn { player: PlayerId },
    Clear { location: LocationKey },
    NotifySummon { card: CardKey },
    Destroy { card: CardKey },
}

impl Instruction {
    #[async_recursion]
    pub async fn process(self, state: &mut GameState, comm: &mut GameCommunicator) -> Result<()> {
        match &self {
            Instruction::StartGame {} => comm.send_game_instruction(state, &self).await,
            Instruction::AddLandscapeSlot { player, index: _index, location_id: lid } => {
                let new_loc = CardSlot::new();
                let loc_key = state.add_location(*lid, Box::new(new_loc));
                let side = match player {
                    PlayerId::Player1 => &mut state.board.side_1,
                    PlayerId::Player2 => &mut state.board.side_2,
                };
                side.field.push(loc_key);
                comm.send_game_instruction(state, &self).await
            }
            Instruction::SetThaum { player: player_id, amount, } => {
                state.get_player_mut(*player_id).thaum = *amount;
                comm.send_game_instruction(state, &self).await
            }
            Instruction::MoveCard { card, to } => {
                let mut card_instance = state.card_instances.get_mut(*card).context("Card instance not found while attempting a move")?;
                let from = card_instance.location;
                let from_instance = state.locations.get_mut(from).unwrap();
                from_instance.remove_card(*card);
                card_instance.location = to.clone();
                let to_instance = state
                    .locations
                    .get_mut(*to)
                    .context("Tried to move a card to a location that doesn't exist")?;
                to_instance.add_card(*card);

                let from_side1 = state.board.side_1.field.contains(&from);
                let from_side2 = state.board.side_2.field.contains(&from);
                let to_side1 = state.board.side_1.field.contains(&to);
                let to_side2 = state.board.side_2.field.contains(&to);

                if to_side1 || to_side2 {
                    if from_side1 != to_side1 || from_side2 != to_side2 {
                        let owner = card_instance.owner;
                        let mut context = TriggerContext::new();
                        context.add_card(state, *card);
                        state.trigger_card_events(owner, comm, BehaviorTrigger::EnterLandscape, &TriggerContext::new())?;
                    }
                }

                comm.send_game_instruction(state, &self).await
            }
            Instruction::DrawCard { player } => {
                let player = state.get_player(*player);
                let deck = state.locations.get(player.deck).unwrap();
                // Check if card is already there
                let card = deck.get_card();
                match card {
                    Some(card) => {
                        Instruction::MoveCard {
                            card,
                            to: player.hand,
                        }.process(state, comm).await
                    }
                    None => {
                        todo!("instantly die")
                    }
                }
            }
            Instruction::CreateCard { id, iid, location, player, } => {
                let loc = state
                    .locations
                    .get_mut(*location)
                    .context("Tried to create a card to a location that does not exist")?;
                let mut card = match state.card_registry.create_card(&id, *iid, *player) {
                    Ok(card) => card,
                    Err(e) => {
                        eprintln!("{e}");
                        return Ok(());
                    }
                };
                card.location = location.clone();
                let key = state.card_instances.insert(card);
                state.card_instances.get_mut(key).unwrap().key = key;
                state.card_keys.insert(*iid, key);
                loc.add_card(key);

                
                if state.board.get_relevant_landscape(state, key).is_some() {
                    let mut context = TriggerContext::new();
                    context.add_card(state, key);
                    state.trigger_card_events(*player, comm, BehaviorTrigger::Summon, &context)?;
                    state.trigger_card_events(*player, comm, BehaviorTrigger::EnterLandscape, &context)?;
                }

                comm.send_game_instruction(state, &self).await
            }
            Instruction::SetTurn { player } => {
                state.current_turn = *player;
                comm.send_game_instruction(state, &self).await
            }
            Instruction::Clear { location } => {
                state.locations.get_mut(*location).context("Tried to clear a non existent location")?.clear();
                comm.send_game_instruction(state, &self).await
            }
            Instruction::NotifySummon { card: card_key } => {
                let card = state.card_instances.get(*card_key).context("Card instance not found during notify summon")?;
                let mut context = TriggerContext::new();
                context.add_card(state, *card_key);
                state.trigger_card_events(card.owner, comm, BehaviorTrigger::Summon, &context)
            }
            Instruction::Destroy { card } => {
                let card_instance = state.card_instances.get(*card).unwrap();
                Instruction::MoveCard { card: *card, to: state.get_side(card_instance.owner).graveyard }.process(state, comm).await
            }
        }
    }

    pub fn build(self, state: &mut GameState) -> Result<String> {
        Ok(match self {
            Instruction::StartGame {} => {
                format!("stg")
            }
            Instruction::AddLandscapeSlot {
                player, index, location_id: lid
            } => format!(
                "ads{}{}{}",
                Tag::Player(player).build(state)?,
                Tag::Integer(index).build(state)?,
                Tag::ServerInstanceId(lid).build(state)?,
            ),
            Instruction::SetThaum {
                player: player_id,
                amount,
            } => format!(
                "sth{}{}",
                Tag::Player(player_id).build(state)?,
                Tag::Integer(amount).build(state)?
            ),
            Instruction::MoveCard {
                card, to
            } => format!(
                "mve{}{}",
                Tag::CardInstance(card).build(state)?,
                Tag::Location(to).build(state)?
            ),
            Instruction::CreateCard {
                id: _id,
                iid,
                location,
                player,
            } => {
                let card_key = state.card_keys.get(&iid).unwrap().clone();
                format!(
                    "crt{}{}{}{}",
                    Tag::CardData(state.card_instances.get(card_key).unwrap().clone()).build(state)?,
                    Tag::ServerInstanceId(iid).build(state)?,
                    Tag::Player(player).build(state)?,
                    Tag::Location(location).build(state)?,
                )
            }
            Instruction::SetTurn {
                player
            } => format!(
                "stt{}",
                Tag::Player(player).build(state)?
            ),
            Instruction::Clear {
                location
            } => format!(
                "clr{}",
                Tag::Location(location).build(state)?
            ),
            _ => todo!("instruction not implemented"),
        })
    }
}
