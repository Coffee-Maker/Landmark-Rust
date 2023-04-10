use std::collections::VecDeque;
use color_eyre::eyre::{ContextCompat, eyre};
use crate::game::tokens::token_deserializer::{TokenBehaviorTriggerWhenName as TriggerState, TokenBehaviorTriggerWhenName};
use crate::game::game_communicator::GameCommunicator;
use crate::game::id_types::{location_ids, LocationId, TokenInstanceId};
use crate::game::id_types::PlayerId;
use crate::game::prompts::PromptCallback;
use crate::game::state_resources::{StateResources, ThreadSafeLocation};
use crate::game::tag::get_tag;
use crate::game::game_context::{GameContext, ContextValue, context_keys};

use color_eyre::Result;
use crate::game::animation_presets::AnimationPreset;
use crate::game::board::Board;
use crate::game::tokens::token_behaviors;
use crate::game::instruction::InstructionToClient;
use crate::game::locations::token_collection::TokenCollection;
use crate::game::locations::token_slot::TokenSlot;
use crate::game::player::Player;

pub type TokenBehaviorTriggerWithContext<'a> = (TriggerState, &'a mut GameContext);

pub struct StateMachine {
    pub state_transition_groups: VecDeque<StateTransitionGroup>,
}

impl StateMachine {
    pub fn new() -> Self {
        Self {
            state_transition_groups: VecDeque::new(),
        }
    }
    
    pub async fn process(&mut self, resources: &mut StateResources, communicator: &mut GameCommunicator) -> Result<Option<PromptCallback>> {
        while let Some(mut next) = self.state_transition_groups.pop_front() {
            while next.queue_empty() == false {
                match next.process(self, resources, communicator).await? {
                    TriggerResult::ReadPrompt(mut prompt) => {
                        prompt.context = next.context.clone();
                        self.state_transition_groups.push_front(next);
                        return Ok(Some(prompt))
                    }
                    _ => {}
                }
            }
        }
        Ok(None)
    }

    pub fn update_current_context(&mut self, context: GameContext) {
        self.state_transition_groups.get_mut(0).unwrap().context = context;
    }
}

impl StateMachine {
    pub async fn start_game(mut self: &mut Self, data: &str, resources: &mut StateResources, communicator: &mut GameCommunicator) -> Result<()> {
        let mut insert_location = |location: Box<ThreadSafeLocation>| {
            resources.locations.insert(location.get_location_id(), location);
        };
        insert_location(Box::new(TokenCollection::new(location_ids::PLAYER_1_SET)));
        insert_location(Box::new(TokenCollection::new(location_ids::PLAYER_1_HAND)));
        insert_location(Box::new(TokenCollection::new(location_ids::PLAYER_2_SET)));
        insert_location(Box::new(TokenCollection::new(location_ids::PLAYER_2_HAND)));
        insert_location(Box::new(TokenSlot::new(location_ids::PLAYER_1_HERO)));
        insert_location(Box::new(TokenSlot::new(location_ids::PLAYER_2_HERO)));
        insert_location(Box::new(TokenSlot::new(location_ids::PLAYER_1_LANDSCAPE)));
        insert_location(Box::new(TokenSlot::new(location_ids::PLAYER_2_LANDSCAPE)));
        insert_location(Box::new(TokenCollection::new(location_ids::PLAYER_1_GRAVEYARD)));
        insert_location(Box::new(TokenCollection::new(location_ids::PLAYER_2_GRAVEYARD)));

        resources.reset_game(communicator).await?;

        // Populate sets
        let set_1_string = get_tag("set1", data)?;
        let set_2_string = get_tag("set2", data)?;
        Player::populate_set(PlayerId::Player1, &set_1_string[..], resources, communicator).await?;
        Player::populate_set(PlayerId::Player2, &set_2_string[..], resources, communicator).await?;

        Player::set_thaum(PlayerId::Player1, resources, 0, communicator).await?;
        Player::prepare_set(PlayerId::Player1, resources, communicator).await?;

        Player::set_thaum(PlayerId::Player2, resources, 0, communicator).await?;
        Player::prepare_set(PlayerId::Player2, resources, communicator).await?;

        Board::prepare_landscapes(resources, communicator).await?;

        for _ in 0..5 {
            self.draw_token(PlayerId::Player1);
            self.draw_token(PlayerId::Player2);
        }

        resources.set_current_turn(resources.current_turn, self, communicator).await?;

        Ok(())
    }

    pub fn move_token(&mut self, token_instance_id: TokenInstanceId, target_location: LocationId) {
        let mut transition_group = StateTransitionGroup::new();

        transition_group.context.insert(context_keys::TOKEN_INSTANCE, ContextValue::TokenInstanceId(token_instance_id));
        transition_group.context.insert(context_keys::TO_LOCATION, ContextValue::LocationId(target_location));
        transition_group.states.push_back(TriggerState::WillBeMoved);
        transition_group.states.push_back(TriggerState::CheckCancel);
        transition_group.states.push_back(TriggerState::HasBeenMoved);
        self.state_transition_groups.push_front(transition_group);
    }

    pub fn summon_token(&mut self, token_instance_id: TokenInstanceId, target_location: LocationId) {
        let mut transition_group = StateTransitionGroup::new();

        transition_group.context.insert(context_keys::TOKEN_INSTANCE, ContextValue::TokenInstanceId(token_instance_id));
        transition_group.context.insert(context_keys::TO_LOCATION, ContextValue::LocationId(target_location));
        transition_group.states.push_back(TriggerState::WillBeMoved);
        transition_group.states.push_back(TriggerState::WillBeSummoned);
        transition_group.states.push_back(TriggerState::HasBeenMoved);
        transition_group.states.push_back(TriggerState::HasBeenSummoned);
        self.state_transition_groups.push_front(transition_group);
    }

    pub fn attack(&mut self, attacker: TokenInstanceId, defender: TokenInstanceId, is_counter_attack: bool) {
        let mut transition_group = StateTransitionGroup::new();
        transition_group.context.insert(context_keys::ATTACKER, ContextValue::TokenInstanceId(attacker));
        transition_group.context.insert(context_keys::DEFENDER, ContextValue::TokenInstanceId(defender));
        transition_group.context.insert(context_keys::IS_COUNTER_ATTACK, ContextValue::Bool(is_counter_attack));
        transition_group.states.push_back(TriggerState::WillAttack);
        transition_group.states.push_back(TriggerState::WillBeAttacked);
        transition_group.states.push_back(TriggerState::CheckCancel);
        transition_group.states.push_back(TriggerState::HasAttacked);
        transition_group.states.push_back(TriggerState::HasBeenAttacked);
        if is_counter_attack {
            self.state_transition_groups.push_back(transition_group);
        } else {
            self.state_transition_groups.push_front(transition_group);
        }
    }

    pub fn deal_effect_damage(&mut self, attacker: TokenInstanceId, defender: TokenInstanceId, amount: i32) {
        let mut transition_group = StateTransitionGroup::new();
        transition_group.context.insert(context_keys::ATTACKER, ContextValue::TokenInstanceId(attacker));
        transition_group.context.insert(context_keys::DEFENDER, ContextValue::TokenInstanceId(defender));
        transition_group.context.insert(context_keys::EFFECT_DAMAGE, ContextValue::I64(amount as i64));
        transition_group.states.push_back(TriggerState::WillBeEffectDamaged);
        transition_group.states.push_back(TriggerState::HasBeenEffectDamaged);
        self.state_transition_groups.push_front(transition_group);
    }

    pub fn defeat(&mut self, attacker: TokenInstanceId, defender: TokenInstanceId) {
        let mut transition_group = StateTransitionGroup::new();
        transition_group.context.insert(context_keys::ATTACKER, ContextValue::TokenInstanceId(attacker));
        transition_group.context.insert(context_keys::DEFENDER, ContextValue::TokenInstanceId(defender));
        transition_group.states.push_back(TriggerState::WillDefeat);
        transition_group.states.push_back(TriggerState::WillBeDefeated);
        transition_group.states.push_back(TriggerState::WillBeDestroyed);
        transition_group.states.push_back(TriggerState::CheckCancel);
        transition_group.states.push_back(TriggerState::HasDefeated);
        transition_group.states.push_back(TriggerState::HasBeenDefeated);
        transition_group.states.push_back(TriggerState::HasBeenDestroyed);
        self.state_transition_groups.push_front(transition_group);
    }

    pub fn destroy_token(&mut self, attacker: TokenInstanceId, defender: TokenInstanceId) {
        let mut transition_group = StateTransitionGroup::new();
        transition_group.context.insert(context_keys::ATTACKER, ContextValue::TokenInstanceId(attacker));
        transition_group.context.insert(context_keys::DEFENDER, ContextValue::TokenInstanceId(defender));
        transition_group.states.push_back(TriggerState::WillBeDestroyed);
        transition_group.states.push_back(TriggerState::CheckCancel);
        transition_group.states.push_back(TriggerState::HasBeenDestroyed);
        self.state_transition_groups.push_front(transition_group);
    }

    pub fn create_token(&mut self, token_id: &str, owner: PlayerId, location: LocationId) {
        let mut transition_group = StateTransitionGroup::new();
        transition_group.context.insert(context_keys::CREATING_TOKEN, ContextValue::String(token_id.to_string()));
        transition_group.context.insert(context_keys::PLAYER, ContextValue::PlayerId(owner));
        transition_group.context.insert(context_keys::TO_LOCATION, ContextValue::LocationId(location));
        transition_group.states.push_back(TriggerState::HasBeenCreated);
        self.state_transition_groups.push_front(transition_group);
    }

    pub fn draw_token(&mut self, player: PlayerId) {
        let mut transition_group = StateTransitionGroup::new();
        transition_group.context.insert(context_keys::PLAYER, ContextValue::PlayerId(player));
        transition_group.states.push_back(TriggerState::WillDrawToken);
        transition_group.states.push_back(TriggerState::CheckCancel);
        transition_group.states.push_back(TriggerState::HasDrawnToken);
        transition_group.states.push_back(TriggerState::HasBeenDrawn);
        self.state_transition_groups.push_front(transition_group);
    }

    pub fn equip_item(&mut self, unit: TokenInstanceId, item: TokenInstanceId) {
        let mut transition_group = StateTransitionGroup::new();

        transition_group.context.insert(context_keys::EQUIP_TARGET, ContextValue::TokenInstanceId(unit));
        transition_group.context.insert(context_keys::EQUIPPING_ITEM, ContextValue::TokenInstanceId(item));
        transition_group.states.push_back(TriggerState::WillBeEquipped);
        transition_group.states.push_back(TriggerState::WillEquip);
        transition_group.states.push_back(TriggerState::CheckCancel);
        transition_group.states.push_back(TriggerState::HasBeenEquipped);
        transition_group.states.push_back(TriggerState::HasEquipped);
        self.state_transition_groups.push_front(transition_group);
    }
}

pub enum TriggerResult {
    Ok,
    TerminateGroup,
    ReadPrompt(PromptCallback)
}

pub struct StateTransitionGroup {
    pub states: VecDeque<TriggerState>,
    pub context: GameContext,
}

impl StateTransitionGroup {
    pub fn new() -> Self {
        Self {
            states: VecDeque::new(),
            context: GameContext::new(),
        }
    }

    pub async fn process(&mut self, state: &mut StateMachine, resources: &mut StateResources, communicator: &mut GameCommunicator) -> Result<TriggerResult> {
        let next = self.states.pop_front().unwrap();
        let result = Ok(match next {
            TriggerState::CheckCancel => {
                let cancel = self.context.get(context_keys::CANCEL).map_or(false, |v| v.as_bool().unwrap());
                if cancel { TriggerResult::TerminateGroup } else { TriggerResult::Ok }
            }

            TriggerState::HasBeenCreated => {
                let token_id = self.context.get(context_keys::CREATING_TOKEN)?.as_string()?;
                let location = self.context.get(context_keys::TO_LOCATION)?.as_location_id()?;
                let owner = self.context.get(context_keys::PLAYER)?.as_player_id()?;
                let instance_id = resources.create_token(token_id, location, owner, communicator).await?;
                self.context.insert(context_keys::CREATING_TOKEN, ContextValue::TokenInstanceId(instance_id));
                TriggerResult::Ok
            }

            TriggerState::WillBeMoved => {
                TriggerResult::Ok
            }
            TriggerState::WillBeSummoned => {
                TriggerResult::Ok
            }
            TriggerState::HasBeenMoved => {
                resources.move_token(
                    self.context.get(context_keys::TOKEN_INSTANCE)?.as_token_instance_id()?,
                    self.context.get(context_keys::TO_LOCATION)?.as_location_id()?,
                    Some(AnimationPreset::EaseInOut),
                    communicator).await?;
                TriggerResult::Ok
            }
            TriggerState::HasBeenSummoned => {
                let token = resources.token_instances.get(&self.context.get(context_keys::TOKEN_INSTANCE)?.as_token_instance_id()?).unwrap();
                let cost = token.cost;
                let token_instance_id = token.instance_id;
                communicator.send_game_instruction(InstructionToClient::Reveal { token: token_instance_id }).await?;
                Player::spend_thaum(token.owner, resources, token.cost, communicator).await?;
                resources.add_equipment_slot(token_instance_id, communicator).await?;
                TriggerResult::Ok
            }

            TriggerState::WillAttack => {
                TriggerResult::Ok
            }
            TriggerState::WillBeAttacked => {
                TriggerResult::Ok
            }
            TriggerState::HasAttacked => {
                let attacker_id = self.context.get(context_keys::ATTACKER)?.as_token_instance_id()?;
                let defender_id = self.context.get(context_keys::DEFENDER)?.as_token_instance_id()?;
                let defender = resources.token_instances.get_mut(&defender_id).unwrap();
                communicator.send_game_instruction(InstructionToClient::Animate {
                    token: attacker_id,
                    location: defender.location,
                    duration: 0.5,
                    preset: AnimationPreset::Attack,
                }).await?;
                TriggerResult::Ok
            }
            TriggerState::HasBeenAttacked => {
                let attacker_id = self.context.get(context_keys::ATTACKER)?.as_token_instance_id()?;
                let defender_id = self.context.get(context_keys::DEFENDER)?.as_token_instance_id()?;
                let attacker_stats = resources.token_instances.get(&attacker_id).unwrap().current_stats;
                let defender = resources.token_instances.get_mut(&defender_id).unwrap();
                defender.current_stats.process_damage(attacker_stats.attack);
                if defender.current_stats.health == 0 {
                    state.defeat(attacker_id, defender_id);
                } else if self.context.get(context_keys::IS_COUNTER_ATTACK).map_or(false, |v| v.as_bool().unwrap()) == false {
                    state.attack(defender_id, attacker_id, true);
                }

                communicator.send_game_instruction(InstructionToClient::UpdateData { token_data: defender.clone() }).await?;

                TriggerResult::Ok
            }

            TriggerState::WillBeEffectDamaged => {
                TriggerResult::Ok
            }
            TriggerState::HasBeenEffectDamaged => {
                let attacker_id = self.context.get(context_keys::ATTACKER)?.as_token_instance_id()?;
                let defender_id = self.context.get(context_keys::DEFENDER)?.as_token_instance_id()?;
                let damage = self.context.get(context_keys::EFFECT_DAMAGE)?.as_i64()?;
                let defender = resources.token_instances.get_mut(&defender_id).unwrap();

                defender.current_stats.process_damage(damage as i32);
                if defender.current_stats.health == 0 {
                    state.defeat(attacker_id, defender_id);
                }

                communicator.send_game_instruction(InstructionToClient::UpdateData { token_data: defender.clone() }).await?;

                TriggerResult::Ok
            }

            TriggerState::WillBeDefeated => {
                TriggerResult::Ok
            }
            TriggerState::WillBeDestroyed => {
                TriggerResult::Ok
            }
            TriggerState::HasBeenDefeated => {
                TriggerResult::Ok
            }
            TriggerState::HasBeenDestroyed => {
                let defender_id = self.context.get(context_keys::DEFENDER)?.as_token_instance_id()?;
                resources.destroy_token(defender_id, communicator).await?;
                TriggerResult::Ok
            }

            TriggerState::WillDrawToken => {
                TriggerResult::Ok
            }
            TriggerState::HasDrawnToken => {
                let player_id = self.context.get(context_keys::PLAYER)?.as_player_id()?;
                let player_set = resources.get_player(player_id).set;
                let player_hand = resources.get_player(player_id).hand;
                let token = resources.locations.get(&player_set).unwrap().get_token();

                match token {
                    None => {
                        communicator.send_game_instruction(InstructionToClient::EndGame { winner: player_id.opponent() }).await?;
                        return Err(eyre!("Ran out of tokens, game concluded"))
                    }
                    Some(token_key) => {
                        resources.move_token(token_key, player_hand, None, communicator).await?;
                        self.context.insert(context_keys::DRAWN_TOKEN, ContextValue::TokenInstanceId(token_key));
                    }
                }

                TriggerResult::Ok
            }
            TriggerState::HasBeenDrawn => {
                TriggerResult::Ok
            }

            TriggerState::WillBeEquipped => {
                TriggerResult::Ok
            }
            TriggerState::WillEquip => {
                TriggerResult::Ok
            }
            TriggerState::HasBeenEquipped => {
                let unit = self.context.get(context_keys::EQUIP_TARGET)?.as_token_instance_id()?;
                let item = self.context.get(context_keys::EQUIPPING_ITEM)?.as_token_instance_id()?;
                let unit_equipment_slots = resources.token_instances.get_mut(&unit).context("Unable to find unit to equip to")?.equipment_slots.clone();
                let item_instance = resources.token_instances.get_mut(&item).context("Unable to find unit to equip to")?;
                let item_from_location = item_instance.location;
                for slot in unit_equipment_slots {
                    if resources.move_token(item, slot, None, communicator).await.is_err() {
                        // Failed, try next slot
                        continue;
                    }
                }

                TriggerResult::Ok
            }
            TriggerState::HasEquipped => {
                TriggerResult::Ok
            }
            _ => TriggerResult::Ok
        });

        if matches!(next, TriggerState::CheckCancel) { return result }

        if let Ok(this) = what_is_this(next.clone()) {
            let this_context_value = self.context.get(&this)?.clone();
            self.context.insert(context_keys::OWNER, ContextValue::PlayerId(resources.token_instances.get(&this_context_value.as_token_instance_id()?).unwrap().owner));
            self.context.insert(context_keys::TRIGGER_THIS, this_context_value);
        } else {
            self.context.insert(context_keys::OWNER, self.context.get(&*who_is_owner(next.clone())?)?.clone());
        }
        for token_id in resources.board.get_tokens_in_play(resources) {
            // Process item triggers first
            let items = resources.token_instances.get(&token_id).unwrap().equipment_slots
                .iter()
                .filter_map(|slot| resources.locations.get(slot).unwrap().get_token())
                .collect::<Vec<TokenInstanceId>>();

            for item in items {
                self.context.insert(context_keys::ACTION_THIS, ContextValue::TokenInstanceId(item));
                token_behaviors::trigger_token_behaviors(
                    item,
                    next.clone(),
                    &mut self.context,
                    state,
                    resources,
                    communicator).await?;
            }


            self.context.insert(context_keys::ACTION_THIS, ContextValue::TokenInstanceId(token_id));
            token_behaviors::trigger_token_behaviors(
                token_id,
                next.clone(),
                &mut self.context,
                state,
                resources,
                communicator).await?;
        }

        return result;
    }

    pub fn queue_empty(&self) -> bool {
        self.states.is_empty()
    }
}

fn what_is_this(state: TriggerState) -> Result<String> {
    Ok(match state {
        TriggerState::WillDestroy => context_keys::ATTACKER,
        TriggerState::WillBeDestroyed => context_keys::DEFENDER,
        TriggerState::HasDestroyed => context_keys::ATTACKER,
        TriggerState::HasBeenDestroyed => context_keys::DEFENDER,
        TriggerState::WillBeMoved => context_keys::TOKEN_INSTANCE,
        TriggerState::HasBeenMoved => context_keys::TOKEN_INSTANCE,
        TriggerState::WillBeSummoned => context_keys::TOKEN_INSTANCE,
        TriggerState::HasBeenSummoned => context_keys::TOKEN_INSTANCE,
        TriggerState::WillAttack => context_keys::ATTACKER,
        TriggerState::WillBeAttacked => context_keys::DEFENDER,
        TriggerState::HasAttacked => context_keys::ATTACKER,
        TriggerState::HasBeenAttacked => context_keys::DEFENDER,
        TriggerState::TookDamage => context_keys::DEFENDER,
        TriggerState::WillBeEffectDamaged => context_keys::DEFENDER,
        TriggerState::HasBeenEffectDamaged => context_keys::DEFENDER,
        TriggerState::WillDefeat => context_keys::ATTACKER,
        TriggerState::WillBeDefeated => context_keys::DEFENDER,
        TriggerState::HasDefeated => context_keys::ATTACKER,
        TriggerState::HasBeenDefeated => context_keys::DEFENDER,
        TriggerState::WillLeaveLandscape => context_keys::TOKEN_INSTANCE,
        TriggerState::HasLeftLandscape => context_keys::TOKEN_INSTANCE,
        TriggerState::WillEnterLandscape => context_keys::TOKEN_INSTANCE,
        TriggerState::HasEnteredLandscape => context_keys::TOKEN_INSTANCE,
        TriggerState::WillEquip => context_keys::EQUIP_TARGET,
        TriggerState::HasEquipped => context_keys::EQUIP_TARGET,
        TriggerState::WillBeEquipped => context_keys::EQUIP_TARGET,
        TriggerState::HasBeenEquipped => context_keys::EQUIPPING_ITEM,
        TriggerState::HasBeenDrawn => context_keys::DRAWN_TOKEN,
        TriggerState::HasBeenCreated => context_keys::CREATING_TOKEN,
        _ => return Err(eyre!("Can't specify what this is for state: {state:?}")),
    }.to_string())
}

fn who_is_owner(state: TriggerState) -> Result<String> {
    Ok(match state {
        TriggerState::WillDrawToken => context_keys::PLAYER,
        TriggerState::HasDrawnToken => context_keys::PLAYER,
        _ => return Err(eyre!("Can't specify what owner is for state: {state:?}")),
    }.to_string())
}