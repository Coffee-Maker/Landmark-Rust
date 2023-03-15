use crate::game::location::Location;

pub struct CardSlot {
    pub lid : u32,
}

impl Location for CardSlot {
    fn set_lid(&mut self, lid: u32) {
        self.lid = lid;
    }

    fn get_lid(&self) -> u32 {
        self.lid
    }
}