use std::fmt::{Display, Formatter};
use std::str::FromStr;
use std::num::ParseIntError;

pub type ServerInstanceId = u64;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct LocationId(pub ServerInstanceId);

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
pub struct CardInstanceId(pub ServerInstanceId);

impl FromStr for CardInstanceId {
    type Err = ParseIntError;

    fn from_str(s: &str) -> color_eyre::Result<Self, Self::Err> {
        Ok(Self(s.parse::<ServerInstanceId>()?))
    }
}

impl Display for CardInstanceId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlayerId {
    Player1 = 0,
    Player2 = 1,
}

impl Display for PlayerId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            player_1 => write!(f, "Player 1"),
            player_2 => write!(f, "Player 2"),
        }
    }
}
