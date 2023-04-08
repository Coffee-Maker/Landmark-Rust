use std::collections::{HashMap, VecDeque};

use color_eyre::eyre::{Context, ContextCompat, eyre};
use color_eyre::Result;
use futures_util::FutureExt;
use crate::TOKEN_REGISTRY;
use crate::game::animation_presets::AnimationPreset;

use crate::game::board::Board;
use crate::game::tokens;
use crate::game::tokens::token_deserializer::{TokenBehavior, TokenBehaviorAction, TokenBehaviorTriggerWhenName, TokenCategory};
use crate::game::tokens::token_instance::TokenInstance;
use crate::game::game_communicator::GameCommunicator;
use crate::game::id_types::{TokenInstanceId, location_ids, LocationId, PlayerId, ServerInstanceId};
use crate::game::instruction::InstructionToClient;
use crate::game::location::Location;
use crate::game::new_state_machine::{StateMachine, StateTransitionGroup};
use crate::game::player::Player;
use crate::game::game_context::{context_keys, ContextValue, GameContext};
use crate::game::prompts::{PromptCallback, PromptInstance, PromptCallbackResult, PromptProfile, PromptType};
use crate::game::tag::get_tag;

pub type ThreadSafeLocation = dyn Location + Send + Sync;

pub struct StateResources {
    pub locations: HashMap<LocationId, Box<ThreadSafeLocation>>,
    pub token_instances: HashMap<TokenInstanceId, TokenInstance>,
    pub round: u32,
    pub player_1: Player,
    pub player_2: Player,
    pub current_turn: PlayerId,
    pub board: Board,
    location_counter: ServerInstanceId,
}

impl StateResources {
    pub fn new() -> Self {
        Self {
            locations: HashMap::new(),
            token_instances: HashMap::new(),
            round: 0,
            player_1: Player::new(PlayerId::Player1, location_ids::PLAYER_1_SET, location_ids::PLAYER_1_HAND),
            player_2: Player::new(PlayerId::Player2, location_ids::PLAYER_2_SET, location_ids::PLAYER_2_HAND),
            current_turn: if fastrand::bool() { PlayerId::Player1 } else { PlayerId::Player2 },
            board: Board::new(),
            location_counter: 0,
        }
    }

    pub async fn reset_game(&mut self, communicator: &mut GameCommunicator) -> Result<()> {
        for key in self.locations.iter().map(|(key, _)| key.clone()).collect::<Vec<_>>() {
            self.clear_location(key, communicator).await?;
        }
        Ok(())
    }

    pub async fn clear_location(&mut self, location: LocationId, communicator: &mut GameCommunicator) -> Result<()> {
        self.locations.get_mut(&location).context("Tried to clear a non existent location")?.clear();
        communicator.send_game_instruction(InstructionToClient::ClearLocation { location }).await
    }

    pub async fn move_token(&mut self, token_instance_id: TokenInstanceId, to: LocationId, animation: Option<AnimationPreset>, communicator: &mut GameCommunicator) -> Result<()> {
        let mut token_instance = self.token_instances.get_mut(&token_instance_id).context("Token instance not found while attempting a move")?;
        let from = token_instance.location;
        let from_instance = self.locations.get_mut(&from).context("Tried to move token from a location that doesn't exist")?;
        from_instance.remove_token(token_instance_id);
        token_instance.location = to.clone();

        let to_instance = self
            .locations
            .get_mut(&to)
            .context("Tried to move a token to a location that doesn't exist")?;
        to_instance.add_token(token_instance_id)?;
        let to_id = to_instance.get_location_id();

        if let Some(animation) = animation {
            communicator.send_game_instruction(InstructionToClient::Animate {
                token: token_instance_id,
                location: to,
                duration: 0.5,
                preset: animation,
            }).await?;
        }

        communicator.send_game_instruction(InstructionToClient::MoveToken {
            token: token_instance_id,
            to: to_id
        }).await?;

        // Check if a unit changed landscapes
        let old_location = location_ids::identify_location(from)?;
        let new_location = location_ids::identify_location(to)?;

        if old_location.is_field() == false && new_location.is_field() {
            token_instance.hidden = false;
            if token_instance.hidden == false {
                communicator.send_game_instruction(InstructionToClient::Reveal { token: token_instance_id }).await?;
            }
        }

        Ok(())
    }

    pub async fn create_token(&mut self, id: &str, location: LocationId, owner: PlayerId, communicator: &mut GameCommunicator) -> Result<TokenInstanceId> {
        let token_instance_id = TokenInstanceId(fastrand::u64(..));

        let loc = self.locations
            .get_mut(&location)
            .context("Tried to create a token to a location that does not exist")?;

        let mut token = match (*TOKEN_REGISTRY.lock().await).instance_token(&id, token_instance_id, location, owner) {
            Ok(token) => token,
            Err(e) => {
                eprintln!("{e}");
                return Err(e);
            }
        };
        token.instance_id = token_instance_id;
        token.location = location;

        communicator.send_game_instruction(InstructionToClient::CreateToken {
            token_data: token.clone(),
            instance_id: token_instance_id,
            player_id: owner,
            location_id: loc.get_location_id()
        }).await?;

        communicator.send_game_instruction(InstructionToClient::UpdateBehaviors { token_data: token.clone() }).await?;

        self.token_instances.insert(token_instance_id, token);
        loc.add_token(token_instance_id)?;

        Ok(token_instance_id)
    }

    pub async fn destroy_token(&mut self, token_instance_id: TokenInstanceId, communicator: &mut GameCommunicator) -> Result<()> {
        let token_instance = self.token_instances.get(&token_instance_id).unwrap();
        if matches!(token_instance.token_data.token_category, TokenCategory::Hero { .. }) {
            communicator.send_game_instruction(InstructionToClient::EndGame { winner: token_instance.owner.opponent() }).await?;
            return Err(eyre!("Game has concluded"))
        }

        let graveyard = self.board.get_side(token_instance.owner).graveyard;
        self.move_token(token_instance_id, graveyard, Some(AnimationPreset::EaseInOut), communicator).await?;

        Ok(())
    }

    pub fn get_player(&self, id: PlayerId) -> &Player {
        match id {
            PlayerId::Player1 => &self.player_1,
            PlayerId::Player2 => &self.player_2,
        }
    }

    pub fn get_player_mut(&mut self, id: PlayerId) -> &mut Player {
        match id {
            PlayerId::Player1 => &mut self.player_1,
            PlayerId::Player2 => &mut self.player_2,
        }
    }

    pub async fn show_selectable_tokens(&self, communicator: &mut GameCommunicator) -> Result<PromptCallback> {
        let mut callback = PromptCallback::new(|prompt, context, state, resources, communicator| {
            let new_callback = match prompt.prompt {
                PromptType::SelectToken(token_instance_id) => {
                    context.insert(context_keys::SELECTED_TOKEN, ContextValue::TokenInstanceId(token_instance_id));
                    Some(resources.show_attackable_tokens(communicator).now_or_never().context("Failed to run async function")??)
                }
                _ => None
            };
            Ok(PromptCallbackResult::End(new_callback))
        }, true);
        for (id, token) in &self.token_instances {
            if token.owner != self.current_turn || location_ids::identify_location(token.location)?.is_field() == false {
                continue;
            }

            callback.add_prompt(PromptProfile {
                prompt_type: PromptType::SelectToken(*id),
                value: false,
                owner: self.current_turn,
            })
        }
        Ok(callback)
    }

    pub async fn show_attackable_tokens(&mut self, communicator: &mut GameCommunicator) -> Result<PromptCallback> {
        let mut callback = PromptCallback::new(|prompt, context, state, resources, communicator| {
            match prompt.prompt {
                PromptType::AttackToken(token_instance_id) => {
                    let attacker = context.get(context_keys::SELECTED_TOKEN)?.as_token_instance_id()?;
                    state.attack(attacker, token_instance_id, false);
                }
                _ => {}
            }
            Ok(PromptCallbackResult::End(None))
        }, true);

        if self.round == 0 {
            return Ok(callback)
        }

        let mut tokens: Vec<&TokenInstance> = self.token_instances.values().collect();
        tokens.retain(|c| location_ids::identify_location(c.location).unwrap().is_field_of(self.current_turn.opponent()));

        let front_most_row = tokens.iter().fold(100, |current_min, token| {
            location_ids::get_slot_position(token.location, &self.board).unwrap().z.min(current_min)
        });

        tokens.retain(|c| {
            location_ids::get_slot_position(c.location, &self.board).unwrap().z == front_most_row
        });

        if tokens.len() == 0 {
            // No more defending tokens, hero should be attackable
            let hero_slot = self.board.get_side(self.current_turn.opponent()).hero;
            tokens.push(self.token_instances.get(
                &self.locations.get(&hero_slot).context("Hero slot does not exist")?.get_token().context("Hero was not found in hero slot")?).context("Hero does not exist")?);
        }

        for token in tokens{
            callback.add_prompt(PromptProfile {
                prompt_type: PromptType::AttackToken(token.instance_id),
                value: false,
                owner: self.current_turn,
            })
        }
        Ok(callback)
    }

    pub async fn can_player_summon_token(&self, token_instance_id: TokenInstanceId, to_location: LocationId, communicator: &mut GameCommunicator) -> Result<bool> {
        let token_instance = self.token_instances.get(&token_instance_id).context("Unable to find token")?;
        let token_location = token_instance.location.clone();

        if token_instance.location == to_location {
            return Ok(false);
        }

        let mut allow = true;
        if token_instance.owner != self.current_turn {
            communicator.send_error("Can't play token out of turn");
            allow = false;
        }

        if token_instance.location != self.get_player(token_instance.owner).hand {
            communicator.send_error("Can't play token from this location");
            allow = false;
        }

        if self.board.get_side(token_instance.owner).field.contains(&to_location) == false {
            communicator.send_error("Can't play token to this location");
            allow = false;
        }

        if token_instance.cost > self.get_player(self.current_turn).thaum {
            communicator.send_error("Can't play token to this location");
            allow = false;
        }

        if allow == false {
            communicator.send_game_instruction(InstructionToClient::MoveToken { token: token_instance_id, to: token_location }).await?;
        }

        return Ok(allow)
    }

    pub async fn set_current_turn(&mut self, player_id: PlayerId, state: &mut StateMachine, communicator: &mut GameCommunicator) -> Result<()> {
        self.current_turn = player_id;
        self.round += 1;
        communicator.send_game_instruction(InstructionToClient::PassTurn { player_id }).await?;
        self.start_turn(state, communicator).await?;
        Ok(())
    }

    pub async fn start_turn(mut self: &mut Self, state: &mut StateMachine, communicator: &mut GameCommunicator) -> Result<()> {
        let thaum = self.round.div_ceil(2);
        Player::set_thaum(self.current_turn, self, thaum + 10, communicator).await?;
        state.draw_token(self.current_turn);

        // Units recover their base defense
        let mut tokens = self.token_instances.values_mut().collect::<Vec<&mut TokenInstance>>();
        tokens.retain(|c| location_ids::identify_location(c.location).unwrap().is_field());
        tokens.retain(|c| c.owner == self.current_turn);
        for unit in tokens {
            unit.current_stats.defense = unit.base_stats.defense;
            communicator.send_game_instruction(InstructionToClient::Animate {
                token: unit.instance_id,
                location: unit.location,
                duration: 0.2,
                preset: AnimationPreset::Raise,
            }).await?;
            communicator.send_game_instruction(InstructionToClient::UpdateData { token_data: unit.clone() }).await?;
            communicator.send_game_instruction(InstructionToClient::Animate {
                token: unit.instance_id,
                location: unit.location,
                duration: 0.2,
                preset: AnimationPreset::EaseInOut,
            }).await?;
        }

        let hero = match self.current_turn {
            PlayerId::Player1 => &mut self.player_1.hero,
            PlayerId::Player2 => &mut self.player_2.hero,
        };

        let hero = self.token_instances.get_mut(&hero).context("Hero not found")?;

        match hero.token_data.token_category {
            TokenCategory::Hero { health, defense } => hero.current_stats.defense = defense,
            _ => {}
        }

        communicator.send_game_instruction(InstructionToClient::UpdateData { token_data: hero.clone() }).await?;
        Ok(())
    }
}