use crate::modules::{ClerkId, TrainId};

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
}

pub type Result<T> = std::result::Result<T, SystemError>;
