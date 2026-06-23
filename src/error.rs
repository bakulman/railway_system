use crate::modules::{ClerkId, TrainId};

#[derive(Debug)]
pub enum SystemError {
    DatabaseError(String),
    ConfigError(String),

    DuplicateTrain { train_id: TrainId },
    DuplicateClerk { clerk_id: ClerkId },
    TrainNotFound { train_id: TrainId },
    ClerkNotFound { clerk_id: ClerkId },
    SeatInsufficient { train_id: TrainId },
    InvalidRoute { reason: String },
    InvalidPrice,
    SeatConfig { seat_id: u32 },
}

pub type Result<T> = std::result::Result<T, SystemError>;
