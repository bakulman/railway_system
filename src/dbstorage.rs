use sqlx::{PgPool, PgTransaction, Postgres, Transaction, postgres::PgPoolOptions, query};
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
            .max_connections(10)
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
        //开启事务
        let mut tx: PgTransaction<'_> = self
            .pool
            .begin()
            .await
            .map_err(|e| SystemError::DatabaseError(e.to_string()))?;

        //注入悲观锁

        sqlx::query("select 1 from trains where train_id = $1 for update;")
            .bind(train_id.0)
            .execute(&mut *tx)
            .await
            .map_err(|e| SystemError::DatabaseError(format!("高并发获取锁失败: {}", e)))?;

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
                if let sqlx::Error::Database(db_err) = &sqlx_err {
                    match db_err.code().as_deref() {
                        Some("P0002") => {
                            return Err(SystemError::InvalidRoute {
                                reason: format!("from station or to station NOT FOUND"),
                            });
                        }
                        Some("P0003") => {
                            return Err(SystemError::SeatConflict {
                                seat_id: seat_number as u32,
                            });
                        }
                        e => {
                            // todo!()
                        }
                    }
                }
                return Err(SystemError::DatabaseError(sqlx_err.to_string()));
            }
        }
    }

    /// 退票业务
    async fn refund_ticket(&self, train_id: TrainId, seat_number: i32) -> Result<()> {
        // 开启事务
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| SystemError::DatabaseError(e.to_string()))?;

        //执行删除
        let result = sqlx::query("delete from tickets where train_id = $1 and seat_number = $2")
            .bind(train_id.0)
            .bind(seat_number)
            .execute(&mut *tx)
            .await
            .map_err(|e| SystemError::DatabaseError(e.to_string()))?;

        // 业务防御, 如果删除影响的行数是 0, 说明这张票不存在
        if result.rows_affected() == 0 {
            return Err(SystemError::InvalidRoute {
                reason: format!(
                    "退票失败: 未找到车次 {} 座位 {} 的购票流水",
                    train_id.0, seat_number
                ),
            });
        }

        tx.commit()
            .await
            .map_err(|e| SystemError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    /// 调用存储过程 1：统计指定车次在指定时间的销售张数
    pub async fn get_train_sales_stats(&self, train_id: TrainId, timestamp: i32) -> Result<i32> {
        let row: (i32,) = sqlx::query_as("SELECT proc_count_train_sales($1, $2);")
            .bind(train_id.0)
            .bind(timestamp)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| SystemError::DatabaseError(e.to_string()))?;

        Ok(row.0)
    }

    /// 调用存储过程 2：统计指定日期（时间戳起始点）各业务员的销售收入
    pub async fn get_clerk_revenue_stats(
        &self,
        day_start_timestamp: i32,
    ) -> Result<Vec<(i32, i32)>> {
        // sqlx 的 query_as 可以直接把存储过程返回的 TABLE 映射为元组的集合
        let rows: Vec<(i32, i32)> = sqlx::query_as("SELECT * FROM proc_clerk_daily_revenue($1);")
            .bind(day_start_timestamp)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| SystemError::DatabaseError(e.to_string()))?;

        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::train::Train;
    use crate::modules::{ClerkId, StationId, TrainId};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[tokio::test]
    async fn test_high_concurrency_sell_ticket() {
        // 1. 初始化连接池
        let url = "postgres://postgres_admin:PasswordBase123!@127.0.0.1:5433/railway_db";
        let storage = DbStorage::connect(url).await.expect("connect failed!");

        // 环境大扫除（注意：也要顺便清理价格表）
        let _ = sqlx::query("DELETE FROM tickets WHERE train_id = 1;")
            .execute(&storage.pool)
            .await;
        let _ = sqlx::query("DELETE FROM train_prices WHERE train_id = 1;")
            .execute(&storage.pool)
            .await;
        let _ = sqlx::query("DELETE FROM trains WHERE train_id = 1;")
            .execute(&storage.pool)
            .await;
        let _ = sqlx::query("DELETE FROM clerks WHERE clerk_id = 1;")
            .execute(&storage.pool)
            .await;

        let storage = Arc::new(storage);
        let success_cnt = Arc::new(AtomicU32::new(0));
        let fail_cnt = Arc::new(AtomicU32::new(0));

        // 2. 注册员工与车次
        storage
            .add_clerk(ClerkId(1), "888")
            .await
            .expect("fail to add clerk");
        storage
            .add_train(TrainId(1), "888", 3)
            .await
            .expect("fail to add train");

        // 3. 🟢 绝杀修复 1：在买票前，必须配置好这趟车的区间顺序（从 1 站到 2 站，物理顺序为 1 和 2）
        storage
            .set_price(TrainId(1), 1, 2, 5000, 1, 2)
            .await
            .expect("fail to set price configuration");

        let mut join_handles = vec![];

        for _i in 0..100 {
            let storage = Arc::clone(&storage);
            let scnt = Arc::clone(&success_cnt);
            let fcnt = Arc::clone(&fail_cnt);

            let handle = tokio::spawn(async move {
                // 4. 🟢 绝杀修复 2：把原本的 `i` 改成固定的 `7`！让 100 个人同时去抢 7 号座位！
                match storage
                    .sell_ticket(ClerkId(1), TrainId(1), StationId(1), StationId(2), 7, 5000)
                    .await
                {
                    Ok(_) => scnt.fetch_add(1, Ordering::Relaxed),
                    Err(_) => fcnt.fetch_add(1, Ordering::Relaxed),
                }
            });
            join_handles.push(handle);
        }

        for handle in join_handles {
            let _ = handle.await;
        }

        let final_success = success_cnt.load(Ordering::SeqCst);
        let final_err = fail_cnt.load(Ordering::SeqCst);

        println!("final success: {final_success}\n final fail: {final_err}");

        // 5. 工业级自动化把关断言
        assert_eq!(
            final_success, 1,
            "同一区间段的同一个座位，应该只能有 1 个人买成功！"
        );
        assert_eq!(final_err, 99, "应该有 99 个人因为区间重叠冲突被硬核拦截！");
    }
}
