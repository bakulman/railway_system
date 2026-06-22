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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::{ClerkId, StationId, TrainId};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[tokio::test]
    async fn test_high_concurrency_sell_ticket() {
        // 1. 初始化连接池（请确保你的 Docker PostgreSQL 正在运行）
        let url = "postgres://postgres_admin:PasswordBase123!@127.0.0.1:5433/railway_db";
        let storage = DbStorage::connect(url).await.expect("数据库连接失败");

        // 用 Arc 把 storage 包起来，方便在 100 个并发线程之间安全地共享只读引用
        let storage = Arc::new(storage);

        // 2. 清理历史测试数据（防止上一次测试的数据干扰）
        // 这里可以直接执行硬编码的删除，保证测试环境纯净
        let _ = sqlx::query("DELETE FROM tickets WHERE train_id = 888;")
            .execute(&storage.pool)
            .await;
        let _ = sqlx::query("DELETE FROM trains WHERE train_id = 888;")
            .execute(&storage.pool)
            .await;
        let _ = sqlx::query("DELETE FROM clerks WHERE clerk_id = 888;")
            .execute(&storage.pool)
            .await;

        // 3. 初始化测试资产：注册一个只有 3 张票的 G888 车次
        storage.add_clerk(ClerkId(888), "并发测试员").await.unwrap();
        storage.add_train(TrainId(888), "G888", 3).await.unwrap();

        // 4. 准备两个线程安全的“高频计数器”（Atomic 原子类型），用来记录战况
        let success_count = Arc::new(AtomicU32::new(0));
        let seat_err_count = Arc::new(AtomicU32::new(0));

        // 记录所有并发任务的句柄，用于最后等待它们全部结束
        let mut join_handles = vec![];

        println!("🔥 警告：100 个买票请求开始全速冲锋...");

        // 5. 疯狂并发：并发拉起 100 个独立的状态机任务
        for i in 0..100 {
            let storage_clone = Arc::clone(&storage);
            let success_clone = Arc::clone(&success_count);
            let err_clone = Arc::clone(&seat_err_count);

            let handle = tokio::spawn(async move {
                // 所有人都在抢 888 车次，座位号统一传 i
                match storage_clone
                    .sell_ticket(
                        ClerkId(888),
                        TrainId(888),
                        StationId(1),
                        StationId(2),
                        i, // 座位号
                        5000,
                    )
                    .await
                {
                    Ok(_) => {
                        // 抢票成功，原子计数器 + 1
                        success_clone.fetch_add(1, Ordering::SeqCst);
                    }
                    Err(SystemError::SeatInsufficient { .. }) => {
                        // 精准捕获到了触发器抛出的余票不足错误，原子计数器 + 1
                        err_clone.fetch_add(1, Ordering::SeqCst);
                    }
                    Err(other) => {
                        panic!("出现了未预期的其他数据库故障: {:?}", other);
                    }
                }
            });
            join_handles.push(handle);
        }

        // 6. 等待这 100 个冲锋的任务全部打完收工
        for handle in join_handles {
            let _ = handle.await;
        }

        // 7. 打印最终战果
        let final_success = success_count.load(Ordering::SeqCst);
        let final_err = seat_err_count.load(Ordering::SeqCst);
        println!(
            "📊 战果结算 -> 成功出票: {} 张, 拦截超卖: {} 次",
            final_success, final_err
        );

        // 8. 工业级终极断言：检查数据是否绝对一致
        assert_eq!(final_success, 3, "发生了超卖！成功票数竟然不等于 3");
        assert_eq!(final_err, 97, "拦截次数不符合预期");

        println!("🎉 测试通过！并发大闸坚不可摧！");
    }
}
