pub mod clerk;
pub mod ticket;
pub mod train;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct TrainId(pub i32);

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct ClerkId(pub i32);

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct StationId(pub i32);

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct TicketId(pub i32);
