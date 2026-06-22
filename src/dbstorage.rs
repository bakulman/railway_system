use sqlx::{PgPool, PgTransaction, Postgres, Transaction, postgres::PgPoolOptions};
// 提示：你需要根据实际情况在 error.rs 中定义你的 SystemError 枚举
use crate::{
    error::{Result, SystemError},
    modules::{ClerkId, StationId, TrainId},
};

pub struct DbStorage {
    pool: PgPool,
}

impl DbStorage {
    /// 初始化连接池并连接到 PostgreSQL
    /// 连接字符串格式: "postgres://postgres_admin:PasswordBase123!@127.0.0.1:5432/railway_db"
    pub async fn connect(database_url: &str) -> Result<Self> {
        // 提示：使用 sqlx::PgPool::connect(database_url).await 来建立连接池
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
            .map_err(|sqlx_err| {
                SystemError::DatabaseError(format!("数据库连接失败: {}", sqlx_err))
            })?;
        Ok(Self { pool })
    }

    /// 往数据库注册一辆新车次
    /// 边界条件：如果 train_id 或 name 已存在，必须捕获数据库异常，返回 SystemError::DuplicateTrain
    pub async fn add_train(&self, id: TrainId, name: &str, total_seats: i32) -> Result<()> {
        // SQL 提示: INSERT INTO trains (train_id, name, total_seats, remaining_seats) VALUES ($1, $2, $3, $4)
        // 注意：PostgreSQL 的占位符是 $1, $2, $3 ...
        let query_future = sqlx::query("insert into trains (train_id, name, total_seats, remaining_seats) values($1, $2, $3, $4);")
            .bind(id.0)
            .bind(name)
            .bind(total_seats)
            .bind(total_seats);

        match query_future.execute(&self.pool).await {
            Ok(_) => Ok(()),
            Err(sqlx_err) => {
                if let sqlx::Error::Database(db_err) = &sqlx_err {
                    if db_err.code().as_deref() == Some("23505") {
                        return Err(SystemError::DuplicateTrain { train_id: id });
                    }
                }
                Err(SystemError::DatabaseError(sqlx_err.to_string()))
            }
        }
    }

    /// 为指定车次配置区间票价与站点顺序
    /// 边界条件：price_cents 必须大于 0；to_seq 必须大于 from_seq，否则返回 SystemError::InvalidRoute
    pub async fn set_price(
        &self,
        train_id: TrainId,
        from_station: i32,
        to_station: i32,
        price_cents: i32,
        from_seq: i32,
        to_seq: i32,
    ) -> Result<()> {
        // SQL 提示: INSERT INTO train_prices ...
        if price_cents <= 0 {
            return Err(SystemError::InvalidPrice);
        }

        if to_seq <= from_seq {
            return Err(SystemError::InvalidRoute {
                reason: format!("终点{} 不能小于起点{}", to_seq, from_seq),
            });
        }
        let query_feature = sqlx::query("insert into train_prices(train_id, from_station_id, to_station_id, price_cents, from_seq, to_seq) values($1,$2,$3,$4,$5,$6);")
            .bind(train_id.0)
            .bind(from_station)
            .bind(to_station)
            .bind(price_cents)
            .bind(from_seq)
            .bind(to_seq);
        match query_feature.execute(&self.pool).await {
            Ok(_) => return Ok(()),
            Err(sqlx_err) => return Err(SystemError::DatabaseError(sqlx_err.to_string())),
        }
    }

    /// 往数据库注册一个业务员
    pub async fn add_clerk(&self, id: ClerkId, name: &str) -> Result<()> {
        // SQL 提示: INSERT INTO clerks ...
        let query_feature =
            sqlx::query("insert into clerks(clerk_id, name, is_active) values($1,$2,$3);")
                .bind(id.0)
                .bind(name)
                .bind(false);
        match query_feature.execute(&self.pool).await {
            Ok(_) => Ok(()),
            Err(sqlx_err) => {
                if let sqlx::Error::Database(db_err) = &sqlx_err
                    && db_err.code().as_deref() == Some("23505")
                {
                    return Err(SystemError::DuplicateClerk { clerk_id: id });
                }
                return Err(SystemError::DatabaseError(sqlx_err.to_string()));
            }
        }
    }

    pub async fn sell_ticket(
        &self,
        clerk_id: ClerkId,
        train_id: TrainId,
        from_station: StationId,
        to_station: StationId,
        seat_number: i32,
        price_cents: i32,
    ) -> Result<()> {
        let mut tx: PgTransaction<'_> = self
            .pool
            .begin()
            .await
            .map_err(|e| SystemError::DatabaseError(e.to_string()))?;

        let sold_at_timestamp = 1719000000;
        let query_future = sqlx::query("insert into tickets (train_id, clerk_id, from_station_id, to_station_id, seat_number, price_cents, sold_at) values($1, $2, $3, $4, $5, $6, $7);")
            .bind(train_id.0)
            .bind(clerk_id.0)
            .bind(from_station.0)
            .bind(to_station.0)
            .bind(seat_number)
            .bind(price_cents)
            .bind(sold_at_timestamp);

        match query_future.execute(&mut *tx).await {
            Ok(_) => {
                tx.commit()
                    .await
                    .map_err(|e| SystemError::DatabaseError(e.to_string()))?;
                return Ok(());
            }
            Err(sqlx_err) => {
                if let sqlx::Error::Database(db_err) = &sqlx_err
                    && db_err.code().as_deref() == Some("P0001")
                {
                    return Err(SystemError::SeatInsufficient { train_id });
                }
                return Err(SystemError::DatabaseError(sqlx_err.to_string()));
            }
        }
    }
}
