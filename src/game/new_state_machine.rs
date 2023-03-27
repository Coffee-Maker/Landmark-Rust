use std::collections::VecDeque;
use color_eyre::eyre::eyre;
use crate::game::card_collection::CardCollection;
use crate::game::card_slot::CardSlot;
use crate::game::cards::token_deserializer::{CardBehaviorTriggerWhenName as TriggerState, CardBehaviorTriggerWhenName};
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
use crate::game::cards::token_behaviors;
use crate::game::instruction::InstructionToClient;
use crate::game::location::Location;
use crate::game::player::Player;

pub type CardBehaviorTriggerWithContext<'a> = (TriggerState, &'a mut GameContext);

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
        insert_location(Box::new(CardCollection::new(location_ids::PLAYER_1_DECK)));
        insert_location(Box::new(CardCollection::new(location_ids::PLAYER_1_HAND)));
        insert_location(Box::new(CardCollection::new(location_ids::PLAYER_2_DECK)));
        insert_location(Box::new(CardCollection::new(location_ids::PLAYER_2_HAND)));
        insert_location(Box::new(CardSlot::new(location_ids::PLAYER_1_HERO)));
        insert_location(Box::new(CardSlot::new(location_ids::PLAYER_2_HERO)));
        insert_location(Box::new(CardSlot::new(location_ids::PLAYER_1_LANDSCAPE)));
        insert_location(Box::new(CardSlot::new(location_ids::PLAYER_2_LANDSCAPE)));
        insert_location(Box::new(CardCollection::new(location_ids::PLAYER_1_GRAVEYARD)));
        insert_location(Box::new(CardCollection::new(location_ids::PLAYER_2_GRAVEYARD)));

        resources.reset_game(communicator).await?;

        // Populate decks
        let deck_1_string = get_tag("deck1", data)?;
        let deck_2_string = get_tag("deck2", data)?;
        Player::populate_deck(PlayerId::Player1, &deck_1_string[..], resources, communicator).await?;
        Player::populate_deck(PlayerId::Player2, &deck_2_string[..], resources, communicator).await?;

        Player::set_thaum(PlayerId::Player1, resources, 0, communicator).await?;
        Player::prepare_deck(PlayerId::Player1, resources, communicator).await?;

        Player::set_thaum(PlayerId::Player2, resources, 0, communicator).await?;
        Player::prepare_deck(PlayerId::Player2, resources, communicator).await?;

        Board::prepare_landscapes(resources, communicator).await?;

        for _ in 0..5 {
            Player::draw_card(PlayerId::Player1, resources, communicator).await?;
            Player::draw_card(PlayerId::Player2, resources, communicator).await?;
        }

        resources.set_current_turn(resources.current_turn, communicator).await?;

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
        self.state_transition_groups.push_front(transition_group);
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
        transition_group.states.push_back(TriggerState::CheckCancel);
        transition_group.states.push_back(TriggerState::HasDefeated);
        transition_group.states.push_back(TriggerState::HasBeenDefeated);
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
                Player::spend_thaum(token.owner, resources, token.cost, communicator).await?;
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
                    card: attacker_id,
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

                communicator.send_game_instruction(InstructionToClient::UpdateData { card_data: defender.clone() }).await?;

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

                communicator.send_game_instruction(InstructionToClient::UpdateData { card_data: defender.clone() }).await?;

                TriggerResult::Ok
            }

            TriggerState::WillBeDefeated => {
                TriggerResult::Ok
            }
            TriggerState::HasBeenDefeated => {
                let defender_id = self.context.get(context_keys::DEFENDER)?.as_token_instance_id()?;
                resources.destroy_token(defender_id, communicator).await?;
                TriggerResult::Ok
            }
            _ => TriggerResult::Ok
        });

        if let Ok(this) = what_is_this(next.clone()) {
            let this_context_value = self.context.get(&this)?.clone();
            self.context.insert(context_keys::OWNER, ContextValue::PlayerId(resources.token_instances.get(&this_context_value.as_token_instance_id()?).unwrap().owner));
            self.context.insert(context_keys::TRIGGER_THIS, this_context_value);
        } else {
            // In cases that have no "this", we need another way to set the context owner. This is needed for cases like owner:turn_started
        }
        for token_id in resources.board.get_cards_in_play(resources) {
            self.context.insert(context_keys::ACTION_THIS, ContextValue::TokenInstanceId(token_id));
            token_behaviors::trigger_card_behaviors(
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
        _ => return Err(eyre!("Can't specify what this is for state: {state:?}")),
    }.to_string())
}