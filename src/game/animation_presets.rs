#[derive(Clone, Copy)]
pub enum AnimationPreset {
    SelectForAttack,
    Raise,
    EaseInOut,
    Attack,
    TakeDamage
}

impl ToString for AnimationPreset {
    fn to_string(&self) -> String {
        match self {
            AnimationPreset::SelectForAttack => "SelectForAttack",
            AnimationPreset::Raise => "Raise",
            AnimationPreset::EaseInOut => "EaseInOut",
            AnimationPreset::Attack => "Attack",
            AnimationPreset::TakeDamage => "TakeDamage",
        }.to_string()
    }
}