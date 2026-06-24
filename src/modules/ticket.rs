use crate::modules::{ClerkId, StationId, TicketId, TrainId};

pub struct Ticket {
    pub id: TicketId,
    pub train_id: TrainId,
    pub clerk_id: ClerkId,
    pub from_station: StationId,
    pub to_station: StationId,
    pub seat_number: i32,
    pub price_cents: i32,
    pub sold_at: i32,
}
