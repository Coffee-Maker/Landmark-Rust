use std::collections::HashMap;
use color_eyre::eyre::ContextCompat;

use color_eyre::Result;

use crate::game::game_communicator::GameCommunicator;
use crate::game::id_types::{TokenInstanceId, LocationId, PlayerId, PromptInstanceId};
use crate::game::instruction::InstructionToClient;
use crate::game::tag::get_tag;
use crate::game::game_context::GameContext;
use crate::game::new_state_machine::StateMachine;
use crate::game::state_resources::StateResources;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PromptType {
    SelectToken(TokenInstanceId),
    AttackToken(TokenInstanceId),
    SelectFieldSlot(LocationId),
}

// Implement ToString for PromptType using debug formatting
impl ToString for PromptType {
    fn to_string(&self) -> String {
        match self {
            PromptType::SelectToken(_) => "SelectToken",
            PromptType::AttackToken(_) => "AttackToken",
            PromptType::SelectFieldSlot(_) => "SelectFieldSlot",
        }.to_string()
    }
}

impl PromptType {
    pub fn to_instruction_string(&self) -> String {
        match self {
            PromptType::SelectToken(_) => "token_select",
            PromptType::AttackToken(_) => "token_attack",
            PromptType::SelectFieldSlot(_) => "slot_select"
        }.into()
    }
}

pub type PromptCallbackClosure = fn(callback_data: PromptInstance, context: &mut GameContext, state: &mut StateMachine, resources: &mut StateResources, communicator: &mut GameCommunicator) -> Result<PromptCallbackResult>;

pub struct PromptProfile {
    pub prompt_type: PromptType,
    pub value: bool,
    pub owner: PlayerId
}

pub enum PromptCallbackResult {
    Keep,
    End(Option<PromptCallback>)
}

pub struct PromptCallback {
    pub cancelable: bool,
    closure: PromptCallbackClosure,
    prompt_instances: HashMap<PromptInstanceId, PromptProfile>,
    pub context: GameContext
}

impl PromptCallback {
    pub fn new(closure: PromptCallbackClosure, cancelable: bool) -> Self {
        Self {
            cancelable,
            closure,
            prompt_instances: HashMap::new(),
            context: GameContext::new()
        }
    }

    pub fn add_prompt(&mut self, prompt: PromptProfile) {
        self.prompt_instances.insert(PromptInstanceId(fastrand::u64(..)), prompt);
    }

    pub async fn create_instructions(&self, communicator: &mut GameCommunicator) -> Result<()> {
        for (id, prompt) in &self.prompt_instances {
            communicator.send_game_instruction(InstructionToClient::AddPrompt {
                prompt_instance_id: *id,
                prompt_type: prompt.prompt_type,
            }).await?;
        }
        Ok(())
    }

    pub fn execute(&mut self, data: String, context: &mut GameContext, state: &mut StateMachine, resources: &mut StateResources, communicator: &mut GameCommunicator) -> Result<PromptCallbackResult> {
        let prompt_instance_id = PromptInstanceId(get_tag("callback_id", &data)?.parse::<u64>()?);
        let value = get_tag("value", &data)?.parse::<bool>()?;
        let prompt_type = self.prompt_instances.get(&prompt_instance_id).context("Failed to find prompt with given instance id")?.prompt_type;
        (self.closure)(PromptInstance { prompt: prompt_type, value }, context, state, resources, communicator)
    }

    pub async fn cancel(&mut self, communicator: &mut GameCommunicator) -> Result<()> {
        for (id, prompt) in &self.prompt_instances {
            communicator.send_game_instruction(InstructionToClient::RemovePrompt {
                prompt_instance_id: *id,
            }).await?;
        }

        Ok(())
    }
}

pub struct PromptInstance {
    pub prompt: PromptType,
    pub value: bool,
}