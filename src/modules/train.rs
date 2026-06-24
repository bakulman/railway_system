use crate::modules::{StationId, TrainId};

pub struct Train {
    pub id: TrainId,
    pub from_station: StationId,
    pub to_station: StationId,

    pub total_seats: i32,
    pub remaining_seats: i32,
}
