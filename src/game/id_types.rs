use std::fmt::{Display, Formatter};
use std::str::FromStr;
use std::num::ParseIntError;
use crate::game::player::Player;

pub type ServerInstanceId = u64;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct LocationId(pub ServerInstanceId);

pub mod location_ids {
    use color_eyre::eyre::{ContextCompat, eyre};
    use color_eyre::Result;
    use crate::game::board::Board;
    use crate::game::cards::token_deserializer::SlotPosition;

    use crate::game::id_types::{LocationId, PlayerId};
    use crate::game::player::Player;

    pub const PLAYER_1_DECK: LocationId = LocationId(100);
    pub const PLAYER_1_HAND: LocationId = LocationId(101);
    pub const PLAYER_1_HERO: LocationId = LocationId(102);
    pub const PLAYER_1_LANDSCAPE: LocationId = LocationId(103);
    pub const PLAYER_1_GRAVEYARD: LocationId = LocationId(104);
    pub const PLAYER_2_DECK: LocationId = LocationId(200);
    pub const PLAYER_2_HAND: LocationId = LocationId(201);
    pub const PLAYER_2_HERO: LocationId = LocationId(202);
    pub const PLAYER_2_LANDSCAPE: LocationId = LocationId(203);
    pub const PLAYER_2_GRAVEYARD: LocationId = LocationId(204);

    pub fn player_deck_location_id(player: PlayerId, index: u64) -> LocationId {
        if player == PlayerId::Player1 { PLAYER_1_DECK } else { PLAYER_2_DECK }
    }

    pub fn player_hand_location_id(player: PlayerId, index: u64) -> LocationId {
        if player == PlayerId::Player1 { PLAYER_1_HAND } else { PLAYER_2_HAND }
    }

    pub fn player_hero_location_id(player: PlayerId) -> LocationId {
        if player == PlayerId::Player1 { PLAYER_1_HERO } else { PLAYER_2_HERO }
    }

    pub fn player_landscape_location_id(player: PlayerId) -> LocationId {
        if player == PlayerId::Player1 { PLAYER_1_LANDSCAPE } else { PLAYER_2_LANDSCAPE }
    }

    pub fn player_graveyard_location_id(player: PlayerId) -> LocationId {
        if player == PlayerId::Player1 { PLAYER_1_GRAVEYARD } else { PLAYER_2_GRAVEYARD }
    }

    pub fn player_field_location_id(player: PlayerId, index: u64) -> LocationId {
        LocationId(if player == PlayerId::Player1 { 1000 } else { 2000 } + index)
    }

    pub fn get_slot_position(location: LocationId, board: &Board) -> Result<SlotPosition> {
        let location_identity = identify_location(location)?;
        let index = match location_identity {
            LocationIdentity::Player1Field => location.0 - 1000,
            LocationIdentity::Player2Field => location.0 - 2000,
            _ => return Err(eyre!("Tried to get slot position of a location that is not on the field")),
        };

        return Ok(match location_identity {
            LocationIdentity::Player1Field => *board.side_1.field_slot_positions.get(index as usize).context("Tried to get a slot position for a slot index that is out of range")?,
            LocationIdentity::Player2Field => *board.side_1.field_slot_positions.get(index as usize).context("Tried to get a slot position for a slot index that is out of range")?,
            _ => return Err(eyre!("Tried to get slot position of a location that is not on the field")),
        })
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum LocationIdentity {
        Player1Deck,
        Player1Hand,
        Player1Hero,
        Player1Landscape,
        Player1Graveyard,
        Player1Field,
        Player2Deck,
        Player2Hand,
        Player2Hero,
        Player2Landscape,
        Player2Graveyard,
        Player2Field,
    }

    impl LocationIdentity {
        pub fn is_field(&self) -> bool {
            match self {
                LocationIdentity::Player1Field | LocationIdentity::Player2Field => true,
                _ => false
            }
        }

        pub fn is_field_of(&self, player: PlayerId) -> bool {
            match player {
                PlayerId::Player1 => self == &LocationIdentity::Player1Field,
                PlayerId::Player2 => self == &LocationIdentity::Player2Field,
                _ => false,
            }
        }
    }

    pub fn identify_location(location_id: LocationId) -> Result<LocationIdentity> {
        Ok(match location_id {
            PLAYER_1_HAND => LocationIdentity::Player1Hand,
            PLAYER_1_DECK => LocationIdentity::Player1Deck,
            PLAYER_1_HERO => LocationIdentity::Player1Hero,
            PLAYER_1_LANDSCAPE => LocationIdentity::Player1Landscape,
            PLAYER_1_GRAVEYARD => LocationIdentity::Player1Graveyard,
            PLAYER_2_HAND => LocationIdentity::Player2Hand,
            PLAYER_2_DECK => LocationIdentity::Player2Deck,
            PLAYER_2_HERO => LocationIdentity::Player2Hero,
            PLAYER_2_LANDSCAPE => LocationIdentity::Player2Landscape,
            PLAYER_2_GRAVEYARD => LocationIdentity::Player2Graveyard,
            _ => {
                if location_id.0 >= 1000 && location_id.0 < 2000 {
                    LocationIdentity::Player1Field
                } else if location_id.0 >= 2000 && location_id.0 < 3000 {
                    LocationIdentity::Player2Field
                } else {
                    return Err(eyre!("Unable to identify location id: {}", location_id.0));
                }
            }
        })
    }
}

impl Display for LocationId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for LocationId {
    type Err = ParseIntError;

    fn from_str(s: &str) -> color_eyre::Result<Self, Self::Err> {
        Ok(Self(s.parse::<ServerInstanceId>()?))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TokenInstanceId(pub ServerInstanceId);

impl FromStr for TokenInstanceId {
    type Err = ParseIntError;

    fn from_str(s: &str) -> color_eyre::Result<Self, Self::Err> {
        Ok(Self(s.parse::<ServerInstanceId>()?))
    }
}

impl Display for TokenInstanceId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PromptInstanceId(pub ServerInstanceId);

impl FromStr for PromptInstanceId {
    type Err = ParseIntError;

    fn from_str(s: &str) -> color_eyre::Result<Self, Self::Err> {
        Ok(Self(s.parse::<ServerInstanceId>()?))
    }
}

impl Display for PromptInstanceId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlayerId {
    Player1 = 0,
    Player2 = 1,
}

impl PlayerId {
    pub fn opponent(&self) -> PlayerId {
        match self {
            PlayerId::Player1 => PlayerId::Player2,
            PlayerId::Player2 => PlayerId::Player1
        }
    }
}

impl Display for PlayerId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PlayerId::Player1 => write!(f, "Player 1"),
            PlayerId::Player2 => write!(f, "Player 2"),
        }
    }
}
