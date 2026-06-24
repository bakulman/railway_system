use crate::modules::{ClerkId, TrainId};
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

pub type Result<T> = std::result::Result<T, SystemError>;

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
    SeatConflict { seat_id: u32 },
}

impl IntoResponse for SystemError {
    fn into_response(self) -> Response {
        // 10 个分支完美穷尽，严格遵循标准的 HTTP 网络响应语义
        let (status, error_code, message) = match self {
            SystemError::InvalidPrice => (
                StatusCode::BAD_REQUEST,
                "INVALID_PRICE",
                "票价必须大于 0".to_string(),
            ),
            SystemError::InvalidRoute { reason } => {
                (StatusCode::BAD_REQUEST, "INVALID_ROUTE", reason)
            }
            SystemError::DuplicateTrain { train_id } => (
                StatusCode::CONFLICT,
                "DUPLICATE_TRAIN",
                format!("车次 ID {} 已存在, 请勿重复注册", train_id.0),
            ),
            SystemError::DuplicateClerk { clerk_id } => (
                StatusCode::CONFLICT,
                "DUPLICATE_CLERK",
                format!("业务员 ID {} 已存在, 请勿重复注册", clerk_id.0),
            ),
            SystemError::TrainNotFound { train_id } => (
                StatusCode::NOT_FOUND, // 🟢 语义化：404 找不到车次
                "TRAIN_NOT_FOUND",
                format!("未找到 ID 为 {} 的车次", train_id.0),
            ),
            SystemError::ClerkNotFound { clerk_id } => (
                StatusCode::NOT_FOUND, // 🟢 语义化：404 找不到员工
                "CLERK_NOT_FOUND",
                format!("未找到 ID 为 {} 的业务员", clerk_id.0),
            ),
            SystemError::SeatInsufficient { train_id } => (
                StatusCode::BAD_REQUEST,
                "SEAT_INSUFFICIENT",
                format!("车次 {} 总余票不足，出票失败", train_id.0),
            ),
            SystemError::SeatConflict { seat_id } => (
                StatusCode::CONFLICT, // 🟢 语义化：409 区间占座冲突归位，变量精准解包
                "SEAT_CONFLICT",
                format!("座位 {} 在该区间段已被锁定, 请选择其他座位", seat_id),
            ),
            SystemError::ConfigError(cfg_err) => {
                eprintln!("🔥 [Config Error]: {}", cfg_err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "CONFIG_ERROR",
                    "服务器配置错误".to_string(),
                )
            }
            SystemError::DatabaseError(raw_err) => {
                eprintln!("🔥 [SQLx Database Critical Error]: {}", raw_err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_SERVER_ERROR",
                    "数据库繁忙, 请稍后再试".to_string(),
                )
            }
        };

        let body = Json(json!({
            "success": false,
            "error_code": error_code,
            "message": message
        }));

        (status, body).into_response()
    }
}
