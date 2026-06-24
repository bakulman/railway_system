use crate::modules::{StationId, TrainId};
use std::collections::HashMap;

pub struct Train {
    pub id: TrainId,
    pub from_station: StationId,
    pub to_station: StationId,

    pub total_seats: u32,
    pub remaining_seats: u32,
}
