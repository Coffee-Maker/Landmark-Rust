#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HighlightType {
    SelectCard,
    AttackCard,
    SelectFieldSlot,
}

impl HighlightType {
    pub fn to_instruction_string(&self) -> String {
        match self {
            HighlightType::SelectCard => "card_select",
            HighlightType::AttackCard => "card_attack",
            HighlightType::SelectFieldSlot => "slot_select"
        }.into()
    }
}

pub struct HighlightProfile {
    pub highlight_type: HighlightType,
}