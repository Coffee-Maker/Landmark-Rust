use std::collections::VecDeque;
use crate::game::card_collection::CardCollection;
use crate::game::card_slot::CardSlot;
use crate::game::cards::card_deserialization::{CardBehaviorTriggerWhenName as TriggerState, CardBehaviorTriggerWhenName};
use crate::game::game_communicator::GameCommunicator;
use crate::game::id_types::{location_ids, LocationId, TokenInstanceId};
use crate::game::id_types::PlayerId;
use crate::game::prompts::PromptCallback;
use crate::game::state_resources::StateResources;
use crate::game::tag::get_tag;
use crate::game::trigger_context::{GameContext, ContextValue};

use color_eyre::Result;
use crate::game::instruction::InstructionToClient;
use crate::game::player::Player;

pub type CardBehaviorTriggerWithContext<'a> = (TriggerState, &'a mut GameContext);

pub struct StateMachine {
    pub state_transition_groups: VecDeque<StateTransitionGroup>,
}

impl StateMachine {
    pub fn process(&mut self, resources: &mut StateResources) -> Option<PromptCallback> {
        while let Some(mut next) = self.state_transition_groups.pop_front() {
            while next.queue_empty() == false {
                match next.process(resources) {
                    TriggerResult::ReadPrompt(mut prompt) => {
                        prompt.context = next.context.clone();
                        self.state_transition_groups.push_front(next);
                        return Some(prompt)
                    }
                    _ => {}
                }
            }
        }
        None
    }

    pub fn update_current_context(&mut self, context: GameContext) {
        self.state_transition_groups.get_mut(0).unwrap().context = context;
    }
}

impl StateMachine {
    pub async fn start_game(mut self: &mut Self, data: &str, resources: &mut StateResources, communicator: &mut GameCommunicator) -> Result<()> {
        resources.insert_location(Box::new(CardCollection::new(location_ids::PLAYER_1_DECK)));
        resources.insert_location(Box::new(CardCollection::new(location_ids::PLAYER_1_HAND)));
        resources.insert_location(Box::new(CardCollection::new(location_ids::PLAYER_2_DECK)));
        resources.insert_location(Box::new(CardCollection::new(location_ids::PLAYER_2_HAND)));
        resources.insert_location(Box::new(CardSlot::new(location_ids::PLAYER_1_HERO)));
        resources.insert_location(Box::new(CardSlot::new(location_ids::PLAYER_2_HERO)));
        resources.insert_location(Box::new(CardSlot::new(location_ids::PLAYER_1_LANDSCAPE)));
        resources.insert_location(Box::new(CardSlot::new(location_ids::PLAYER_2_LANDSCAPE)));
        resources.insert_location(Box::new(CardCollection::new(location_ids::PLAYER_1_GRAVEYARD)));
        resources.insert_location(Box::new(CardCollection::new(location_ids::PLAYER_2_GRAVEYARD)));
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

        resources.board.prepare_landscapes(resources, communicator).await?;

        for _ in 0..5 {
            Player::draw_card(PlayerId::Player1, resources, communicator).await?;
            Player::draw_card(PlayerId::Player2, resources, communicator).await?;
        }

        // Set random turn
        //resources.set_current_turn(self.starting_player, communicator).await?;

        Ok(())
    }

    pub fn move_token(&mut self, token_instance_id: TokenInstanceId, target_location: LocationId) {
        let mut transition_group = StateTransitionGroup {
            states: VecDeque::new(),
            context: GameContext::new(),
        };

        transition_group.context.insert("token_to_move", ContextValue::TokenInstanceId(token_instance_id));
        transition_group.context.insert("target_location", ContextValue::LocationId(target_location));

        transition_group.states.push_back(CardBehaviorTriggerWhenName::WillBeMoved);
        transition_group.states.push_back(CardBehaviorTriggerWhenName::HasBeenMoved);
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
    pub fn process(&mut self, resources: &mut StateResources) -> TriggerResult {
        let next = self.states.pop().unwrap();
        match next {
            TriggerState::WillBeMoved => {
                TriggerResult::Ok
            }
            TriggerState::HasBeenMoved => {
                let cancel = self.context.get("cancel").unwrap().as_bool().unwrap();
                if cancel {
                    TriggerResult::TerminateGroup
                } else {
                    // Move logic goes here...
                    TriggerResult::Ok
                }
            }
            _ => TriggerResult::Ok
        }
    }

    pub fn queue_empty(&self) -> bool {
        self.states.is_empty()
    }
}
