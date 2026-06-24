mod dbstorage;
mod error;
mod modules;

use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing::post};
use dbstorage::DbStorage;
use modules::{ClerkId, StationId, TicketId, TrainId};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;

use tower_http::cors::{Any, CorsLayer};
// 1. 定义前端 HTTP 请求体(DTO)

#[derive(Deserialize)]
struct CreateClerkRequest {
    clerk_id: i32,
    name: String,
}

#[derive(Deserialize)]
struct CreateTrainRequest {
    train_id: i32,
    name: String,
    total_seats: i32,
}

#[derive(Deserialize)]
struct SetPriceRequest {
    train_id: i32,
    from_station: i32,
    to_station: i32,
    price_cents: i32,
    from_seq: i32,
    to_seq: i32,
}

#[derive(Deserialize)]
struct SellTicketRequest {
    clerk_id: i32,
    train_id: i32,
    from_station_id: i32,
    to_station_id: i32,
    seat_number: i32,
    price_cents: i32,
}

//2. Web 处理器(Handler)

/// API: 注册业务员
async fn handle_add_clerk(
    State(storage): State<Arc<DbStorage>>,
    Json(payload): Json<CreateClerkRequest>,
) -> error::Result<impl IntoResponse> {
    storage
        .add_clerk(ClerkId(payload.clerk_id), &payload.name)
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({"success":true, "message":"业务员注册成功"})),
    ))
}

async fn handle_add_train(
    State(storage): State<Arc<DbStorage>>,
    Json(payload): Json<CreateTrainRequest>,
) -> error::Result<impl IntoResponse> {
    storage
        .add_train(
            TrainId(payload.train_id),
            &payload.name,
            payload.total_seats,
        )
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({"success":true, "message":"Train 注册成功"})),
    ))
}

async fn handle_set_price(
    State(storage): State<Arc<DbStorage>>,
    payload: Json<SetPriceRequest>,
) -> error::Result<impl IntoResponse> {
    storage
        .set_price(
            TrainId(payload.train_id),
            payload.from_station,
            payload.to_station,
            payload.price_cents,
            payload.from_seq,
            payload.to_seq,
        )
        .await?;

    Ok(Json(json!({"success":true, "message":"区间配置成功"})))
}

async fn handle_sell_ticket(
    State(storage): State<Arc<DbStorage>>,
    Json(payload): Json<SellTicketRequest>,
) -> error::Result<impl IntoResponse> {
    // 💡 穿透调用：直接扣动你写好的带悲观锁的 sell_ticket 扳机！
    storage
        .sell_ticket(
            ClerkId(payload.clerk_id),
            TrainId(payload.train_id),
            StationId(payload.from_station_id),
            StationId(payload.to_station_id),
            payload.seat_number,
            payload.price_cents,
        )
        .await?;

    // 一切正常，原子事务已安全提交
    Ok(Json(json!({
        "success": true,
        "message": "出票成功",
        "data": { "seat_number": payload.seat_number }
    })))
}

// 3. 异步主函数

#[tokio::main]
async fn main() {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://postgres_admin:PasswordBase123!@127.0.0.1:5433/railway_db".to_string()
    });
    println!("正在连接数据库...");

    let storage = DbStorage::connect(&database_url)
        .await
        .expect("数据库连接失败, 请检查 docker 状态");

    let shared_storage = Arc::new(storage);

    let app = Router::new()
        .route("/api/v1/clerks", post(handle_add_clerk))
        .route("/api/v1/trains", post(handle_add_train))
        .route("/api/v1/prices", post(handle_set_price))
        .route("/api/v1/tickets/sell", post(handle_sell_ticket))
        // 🟢 极其优雅：通过 with_state 一把将数据底座焊死在整个 Web 路由生命周期中
        .with_state(shared_storage);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();

    println!("铁路系统已安全挂载至 http://127.0.0.1:8080");

    axum::serve(listener, app).await.unwrap();
}
