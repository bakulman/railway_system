use crate::modules::{ClerkId, TicketId, TrainId};

pub struct Ticket {
    pub id: TicketId,
    pub train_id: TrainId,
    pub clerk_id: ClerkId,
    pub from_station: TrainId,
    pub to_station: TrainId,
    pub seat_number: u32,
    pub price_cents: u32,
    pub sold_at: u64,
}
