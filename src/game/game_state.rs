use std::char::MAX;
use std::collections::{HashMap, VecDeque};
use std::fmt::{Display, Formatter};
use std::num::ParseIntError;
use std::str::FromStr;

use color_eyre::eyre;
use color_eyre::eyre::{Context, ContextCompat, eyre};
use eyre::Result;
use futures_util::FutureExt;
use once_cell::sync::Lazy;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::WebSocketStream;
use crate::game::animation_presets::AnimationPreset;

use crate::game::board::Board;
use crate::game::card_collection::CardCollection;
use crate::game::card_slot::CardSlot;
use crate::game::cards::card_behaviors;
use crate::game::cards::card_behaviors::CardBehaviorResult;
use crate::game::cards::card_deserialization::{CardBehaviorTriggerWhenName, CardCategory};
use crate::game::cards::card_instance::CardInstance;
use crate::game::cards::card_registry::CardRegistry;
use crate::game::game_communicator::GameCommunicator;
use crate::game::id_types::{CardInstanceId, location_ids, LocationId, PlayerId, PromptInstanceId, ServerInstanceId};
use crate::game::id_types::PlayerId::{Player1, Player2};
use crate::game::instruction::InstructionToClient;
use crate::game::player::Player;
use crate::game::prompts::{PromptCallback, PromptCallbackClosure, PromptCallbackContext, PromptCallbackResult, PromptProfile, PromptType};
use crate::game::state_resources::StateResources;
use crate::game::tag::get_tag;
use crate::game::trigger_context::{CardBehaviorContext, ContextValue};

pub async fn game_service(websocket: WebSocketStream<TcpStream>) -> Result<()> {
    println!("Starting game service");

    let mut communicator = GameCommunicator::new(websocket);
    let mut game_state = GameState::new();

    let mut current_callback: Option<PromptCallback> = None;

    loop {
        let msg = communicator.read_message().await?;

        let message = msg.into_text().unwrap();

        let [instruction, data] = message.split('|').collect::<Vec<_>>()[..] else {
            println!("Could not execute invalid instruction.");
            continue;
        };

        if let Some(callback) = &mut current_callback {
            if instruction == "callback" {
                match callback.execute(data.to_string(), &mut game_state, &mut communicator)? {
                    PromptCallbackResult::Keep => { },
                    PromptCallbackResult::End(new_callback) => {
                        callback.cancel(&mut communicator).await?;
                        if let Some(callback) = &new_callback {
                            callback.create_instructions(&mut communicator).await?;
                        }
                        current_callback = new_callback;
                        if current_callback.is_none() {
                            let callback = game_state.show_selectable_cards(&mut communicator).await?;
                            callback.create_instructions(&mut communicator).await?;
                            current_callback = Some(callback);
                        }
                    }
                }
                continue;
            }
        }

        let result = match instruction {
            "start_game" => {
                game_state = GameState::new();
                game_state.start_game(data, &mut communicator).await
            },
            "move_card" => {
                if game_state.can_player_move_card(data, &mut communicator).await? {
                    if let Some(callback) = &mut current_callback {
                        callback.cancel(&mut communicator).await?;
                        current_callback = None;
                    }
                    let mut callback = game_state.player_moved_card(data, &mut communicator).await?;
                    if let Some(callback) = callback {
                        callback.create_instructions(&mut communicator).await?;
                        current_callback = Some(callback);
                    }
                }
                Ok(())
            }
            "pass_turn" => {
                let mut cancel = false;
                if let Some(callback) = &mut current_callback {
                    if callback.cancelable {
                        callback.cancel(&mut communicator).await?;
                        current_callback = None;
                    } else {
                        cancel = true;
                    }
                }
                if cancel == false {
                    let callback = game_state.player_pass_turn(data, &mut communicator).await?;
                    callback.create_instructions(&mut communicator).await?;
                    current_callback = Some(callback);
                }
                Ok(())
            },
            _ => Err(eyre!("Unknown instruction: {}", instruction)),
        };

        match result {
            Ok(_) => {

            }
            Err(e) => {
                communicator.send_error(&e.to_string()).await?;
            }
        }

        if current_callback.is_none() {
            let callback = game_state.show_selectable_cards(&mut communicator).await?;
            callback.create_instructions(&mut communicator).await?;
            current_callback = Some(callback);
        }
    }
}

pub type CardBehaviorTriggerWithContext = (CardBehaviorTriggerWhenName, CardBehaviorContext);
pub type CardBehaviorTriggerQueue = VecDeque<CardBehaviorTriggerWithContext>;


pub struct GameState {
    pub current_turn: PlayerId,
    pub starting_player: PlayerId,
    pub round: u32,
    pub player_1: Player,
    pub player_2: Player,
    pub board: Board,
    pub resources: StateResources,
    location_counter: ServerInstanceId,
    pub callback_context: HashMap<String, ContextValue>
}

impl GameState {
    pub fn new() -> Self {
        Self {
            current_turn: Player1,
            starting_player: Player1,
            round: 0,
            player_1: Player::new(Player1, location_ids::PLAYER_1_DECK, location_ids::PLAYER_1_HAND),
            player_2: Player::new(Player2, location_ids::PLAYER_2_DECK, location_ids::PLAYER_2_HAND),
            resources: StateResources::new(),
            board: Board::new(),
            location_counter: 100,
            callback_context: HashMap::new(),
        }
    }

    pub async fn start_game(mut self: &mut Self, data: &str, communicator: &mut GameCommunicator) -> Result<()> {
        self.resources.insert_location(Box::new(CardCollection::new(location_ids::PLAYER_1_DECK)));
        self.resources.insert_location(Box::new(CardCollection::new(location_ids::PLAYER_1_HAND)));
        self.resources.insert_location(Box::new(CardCollection::new(location_ids::PLAYER_2_DECK)));
        self.resources.insert_location(Box::new(CardCollection::new(location_ids::PLAYER_2_HAND)));
        self.resources.insert_location(Box::new(CardSlot::new(location_ids::PLAYER_1_HERO)));
        self.resources.insert_location(Box::new(CardSlot::new(location_ids::PLAYER_2_HERO)));
        self.resources.insert_location(Box::new(CardSlot::new(location_ids::PLAYER_1_LANDSCAPE)));
        self.resources.insert_location(Box::new(CardSlot::new(location_ids::PLAYER_2_LANDSCAPE)));
        self.resources.insert_location(Box::new(CardCollection::new(location_ids::PLAYER_1_GRAVEYARD)));
        self.resources.insert_location(Box::new(CardCollection::new(location_ids::PLAYER_2_GRAVEYARD)));
        self.resources.reset_game(communicator).await?;

        // Populate decks
        let deck_1_string = get_tag("deck1", data)?;
        let deck_2_string = get_tag("deck2", data)?;
        self.player_1.populate_deck(&deck_1_string[..], &mut self.resources, communicator).await?;
        self.player_2.populate_deck(&deck_2_string[..], &mut self.resources, communicator).await?;

        self.player_1.set_thaum( 0, communicator).await?;
        self.player_1.prepare_deck(&mut self.resources, &self.board, communicator).await?;

        self.player_2.set_thaum( 0, communicator).await?;
        self.player_2.prepare_deck(&mut self.resources, &self.board, communicator).await?;

        self.board.prepare_landscapes(&mut self.resources, communicator).await?;

        for _ in 0..5 {
            self.player_1.draw_card(&mut self.resources, communicator).await?;
            self.player_2.draw_card(&mut self.resources, communicator).await?;
        }

        // Set random turn
        self.starting_player = if fastrand::bool() { Player1 } else { Player2 };
        self.set_current_turn(self.starting_player, communicator).await?;

        Ok(())
    }

    pub async fn can_player_move_card(&self, data: &str, communicator: &mut GameCommunicator) -> Result<bool> {
        let card_id = get_tag("card", data)?.parse::<CardInstanceId>()?;
        let target_location_id = get_tag("location", data)?.parse::<LocationId>()?;

        let card_instance = self.resources.card_instances.get(&card_id).context("Unable to find card")?;
        let card_location = card_instance.location.clone();

        if card_instance.location == target_location_id {
            return Ok(false);
        }

        let mut allow = true;
        if card_instance.owner != self.current_turn {
            communicator.send_error("Can't play card out of turn");
            allow = false;
        }

        if card_instance.location != self.get_player(card_instance.owner).hand {
            communicator.send_error("Can't play card from this location");
            allow = false;
        }

        if self.board.get_side(card_instance.owner).field.contains(&target_location_id) == false {
            communicator.send_error("Can't play card to this location");
            allow = false;
        }

        if card_instance.cost > self.get_player(self.current_turn).thaum {
            communicator.send_error("Can't play card to this location");
            allow = false;
        }

        if allow == false {
            communicator.send_game_instruction( InstructionToClient::MoveCard { card: card_id, to: card_location }).await?;
        }

        return Ok(allow)
    }

    pub async fn player_moved_card(&mut self, data: &str, communicator: &mut GameCommunicator) -> Result<Option<PromptCallback>> {
        let card_id = get_tag("card", data)?.parse::<CardInstanceId>()?;
        let target_location_id = get_tag("location", data)?.parse::<LocationId>()?;

        let card_instance = self.resources.card_instances.get(&card_id).context("Unable to find card")?;
        let card_location = card_instance.location.clone();

        let new_player_thaum = self.get_player(card_instance.owner).thaum - card_instance.cost;
        let player = card_instance.owner;

        let queue = self.resources.pre_move_card(card_id, target_location_id, self.current_turn, communicator).await?;
        let behavior_result = card_behaviors::trigger_all_card_behaviors(
            queue,
            self.current_turn,
            self,
            communicator
        ).await?;

        if behavior_result == CardBehaviorResult::Cancel {
            communicator.send_game_instruction( InstructionToClient::MoveCard { card: card_id, to: card_location }).await?;
            return Ok(None)
        }

        self.get_player_mut(player).set_thaum(new_player_thaum, communicator).await?;

        let queue = self.resources.move_card(card_id, target_location_id, self.current_turn, None, communicator).await?;
        card_behaviors::trigger_all_card_behaviors(
            queue,
            self.current_turn,
            self,
            communicator
        ).await?;
        Ok(None)
    }

    pub async fn player_pass_turn(&mut self, data: &str, communicator: &mut GameCommunicator) -> Result<PromptCallback> {
        self.set_current_turn(
            if self.current_turn == PlayerId::Player1 { PlayerId::Player2 } else { PlayerId::Player1 },
            communicator
        ).await
    }

    pub async fn set_current_turn(&mut self, player_id: PlayerId, communicator: &mut GameCommunicator) -> Result<PromptCallback> {
        self.current_turn = player_id;
        if player_id == self.starting_player.opponent() {
            self.round += 1;
        }
        communicator.send_game_instruction(InstructionToClient::PassTurn { player_id }).await?;
        self.start_turn(communicator).await
    }

    pub async fn start_turn(&mut self, communicator: &mut GameCommunicator) -> Result<PromptCallback> {
        let thaum = self.round + 1;
        let player = match self.current_turn {
            Player1 => &mut self.player_1,
            Player2 => &mut self.player_2,
        };
        player.set_thaum(thaum, communicator).await?;
        // Round 0 is the first turn. Attacks shouldn't be declared and a card should not be drawn
        if self.round > 0 {
            player.draw_card(&mut self.resources, communicator).await?;
        }

        // Units recover their base defense
        let mut cards = self.resources.card_instances.values_mut().collect::<Vec<&mut CardInstance>>();
        cards.retain(|c| location_ids::identify_location(c.location).unwrap().is_field());
        cards.retain(|c| c.owner == self.current_turn);
        for unit in cards {
            unit.current_stats.defense = unit.base_stats.defense;
            communicator.send_game_instruction(InstructionToClient::Animate {
                card: unit.instance_id,
                location: unit.location,
                duration: 0.2,
                preset: AnimationPreset::Raise,
            }).await?;
            communicator.send_game_instruction(InstructionToClient::UpdateData { card_data: unit.clone() }).await?;
            communicator.send_game_instruction(InstructionToClient::Animate {
                card: unit.instance_id,
                location: unit.location,
                duration: 0.2,
                preset: AnimationPreset::EaseInOut,
            }).await?;
        }

        let hero = self.resources.card_instances.get_mut(&player.hero).context("Hero not found")?;
        match hero.card.card_category {
            CardCategory::Hero { health, defense } => hero.current_stats.defense = defense,
            _ => {}
        }
        communicator.send_game_instruction(InstructionToClient::UpdateData { card_data: hero.clone() }).await?;

        self.show_selectable_cards(communicator).await
    }

    pub async fn show_selectable_cards(&mut self, communicator: &mut GameCommunicator) -> Result<PromptCallback> {
        let mut callback = PromptCallback::new(|context: PromptCallbackContext, state: &mut GameState, communicator: &mut GameCommunicator| {
            let new_callback = match context.prompt {
                PromptType::SelectCard(card_instance_id) => {
                    state.callback_context.insert("selected_card".to_string(), ContextValue::CardInstance(card_instance_id));
                    Some(state.show_attackable_cards(communicator).now_or_never().context("Failed to run async function")??)
                }
                _ => None
            };
            Ok(PromptCallbackResult::End(new_callback))
        }, true);
        for (id, card) in &self.resources.card_instances {
            if card.owner != self.current_turn || location_ids::identify_location(card.location)?.is_field() == false {
                continue;
            }

            callback.add_prompt(PromptProfile {
                prompt_type: PromptType::SelectCard(*id),
                value: false,
                owner: self.current_turn,
            })
        }
        Ok(callback)
    }

    pub async fn show_attackable_cards(&mut self, communicator: &mut GameCommunicator) -> Result<PromptCallback> {
        let mut callback = PromptCallback::new(|context: PromptCallbackContext, state: &mut GameState, communicator: &mut GameCommunicator| {
            match context.prompt {
                PromptType::AttackCard(card) => {
                    state.declare_attack(
                        state.callback_context.get("selected_card").context("Selected card was not set")?.as_card_instance().context("Selected card was not card instance")?,
                        card, communicator).now_or_never().context("Failed to run async function :(")??; // Todo: Is this correct? It feels danger.
                    },
                _ => {}
            }
            Ok(PromptCallbackResult::End(None))
        }, true);

        if self.round == 0 {
            return Ok(callback)
        }

        let mut cards: Vec<&CardInstance> = self.resources.card_instances.values().collect();
        cards.retain(|c| location_ids::identify_location(c.location).unwrap().is_field_of(self.current_turn.opponent()));

        let front_most_row = cards.iter().fold(100, |current_min, card| {
            location_ids::get_slot_position(card.location, &self.board).unwrap().z.min(current_min)
        });

        cards.retain(|c| {
            location_ids::get_slot_position(c.location, &self.board).unwrap().z == front_most_row
        });

        if cards.len() == 0 {
            // No more defending cards, hero should be attackable
            let hero_slot = self.board.get_side(self.current_turn.opponent()).hero;
            cards.push(self.resources.card_instances.get(
                &self.resources.locations.get(&hero_slot).context("Hero slot does not exist")?.get_card().context("Hero was not found in hero slot")?).context("Hero does not exist")?);
        }

        for card in cards{
            callback.add_prompt(PromptProfile {
                prompt_type: PromptType::AttackCard(card.instance_id),
                value: false,
                owner: self.current_turn,
            })
        }
        Ok(callback)
    }

    pub async fn declare_attack(&mut self, source: CardInstanceId, target: CardInstanceId, communicator: &mut GameCommunicator) -> Result<Option<PromptCallback>> {
        let mut queue = CardBehaviorTriggerQueue::new();
        let mut context = CardBehaviorContext::new(self.current_turn);
        context.insert("this", ContextValue::CardInstance(source));
        context.insert("target", ContextValue::CardInstance(target));
        queue.push_back((CardBehaviorTriggerWhenName::WillAttack, context));

        let mut context = CardBehaviorContext::new(self.current_turn);
        context.insert("this", ContextValue::CardInstance(target));
        context.insert("attacker", ContextValue::CardInstance(source));
        queue.push_back((CardBehaviorTriggerWhenName::WillBeAttacked, context));

        let result = card_behaviors::trigger_all_card_behaviors(queue, self.current_turn, self, communicator).await?;
        match result {
            CardBehaviorResult::Ok => {}
            CardBehaviorResult::Cancel => return Ok(None)
        }

        let mut source_stats = self.resources.card_instances.get(&source).context("Failed to find card instance")?.current_stats;
        let target_instance = self.resources.card_instances.get(&target).context("Failed to find card instance")?;
        let mut target_stats = target_instance.current_stats;
        let target_location = target_instance.location;
        let target_owner = target_instance.owner;

        target_stats.process_damage(source_stats.attack);
        source_stats.process_damage(target_stats.attack);

        self.resources.card_instances.get_mut(&target).unwrap().current_stats = target_stats;
        self.resources.card_instances.get_mut(&source).unwrap().current_stats = source_stats;

        communicator.send_game_instruction(InstructionToClient::Animate {
            card: source,
            location: target_location,
            duration: 1.0,
            preset: AnimationPreset::Attack,
        }).await?;
        communicator.send_game_instruction(InstructionToClient::UpdateData { card_data: self.resources.card_instances.get(&target).unwrap().clone() }).await?;
        communicator.send_game_instruction(InstructionToClient::UpdateData { card_data: self.resources.card_instances.get(&source).unwrap().clone() }).await?;

        let mut queue = CardBehaviorTriggerQueue::new();
        let mut context = CardBehaviorContext::new(self.current_turn);
        context.insert("card_instance", ContextValue::CardInstance(source));
        context.insert("target", ContextValue::CardInstance(target));
        queue.push_back((CardBehaviorTriggerWhenName::HasAttacked, context.clone()));

        let mut context = CardBehaviorContext::new(self.current_turn);
        context.insert("card_instance", ContextValue::CardInstance(target));
        context.insert("damage_dealer", ContextValue::CardInstance(source));
        queue.push_back((CardBehaviorTriggerWhenName::HasBeenAttacked, context));

        card_behaviors::trigger_all_card_behaviors(queue, self.current_turn, self, communicator).await?;

        if target_stats.health == 0 {
            let queue = self.resources.destroy_card(target, communicator).await?;
            card_behaviors::trigger_all_card_behaviors(queue, target_owner, self, communicator).await?;
        }

        if source_stats.health == 0 {
            let queue = self.resources.destroy_card(source, communicator).await?;
            card_behaviors::trigger_all_card_behaviors(queue, target_owner, self, communicator).await?;
        }

        Ok(None)
    }

    pub async fn deal_effect_damage(&mut self, target: CardInstanceId, damage: i32, communicator: &mut GameCommunicator) -> Result<Option<PromptCallback>> {
        let target_instance = self.resources.card_instances.get(&target).context("Failed to find card instance")?;
        let mut target_stats = target_instance.current_stats;
        let target_location = target_instance.location;
        let target_owner = target_instance.owner;

        target_stats.process_damage(damage);

        self.resources.card_instances.get_mut(&target).unwrap().current_stats = target_stats;

        communicator.send_game_instruction(InstructionToClient::UpdateData { card_data: self.resources.card_instances.get(&target).unwrap().clone() }).await?;

        if target_stats.health == 0 {
            let queue = self.resources.destroy_card(target, communicator).await?;
            card_behaviors::trigger_all_card_behaviors(queue, target_owner, self, communicator).await?;
        }

        Ok(None)
    }

    pub fn get_player(&self, id: PlayerId) -> &Player {
        match id {
            Player1 => &self.player_1,
            Player2 => &self.player_2,
        }
    }

    pub fn get_player_mut(&mut self, id: PlayerId) -> &mut Player {
        match id {
            Player1 => &mut self.player_1,
            Player2 => &mut self.player_2,
        }
    }
}
