use crate::modules::{StationId, TrainId};
use std::collections::HashMap;

pub struct Train {
    pub id: TrainId,
    pub name: String,
    pub total_seats: u32,
    pub remaining_seats: u32,
}
