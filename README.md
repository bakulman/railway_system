
# ⚡ Railway-Admiral: 高并发分布式火车票务管理系统

这是一个专为解决春运级高并发抢票场景而设计的火车票务管理系统。项目摒弃了传统 Web 框架的繁琐与臃肿，采用高性能、内存安全的 **Rust (Axum + Tokio)** 构建 Web 异步网关，并深度联调 **PostgreSQL** 关系型数据库。

系统在底层焊死了**行级悲观锁 (`FOR UPDATE`)** 与 **PL/pgSQL 区间段几何重叠触发器**，实现微秒级的高并发防超卖与防重叠占座。同时提供了一个单页面无依赖的实时可视化压测沙盘前端。

---

## 🛠️ 核心技术栈

* **开发语言**: Rust (Edition 2021) — 极致的运行效率与编译期安全保证
* **Web 框架**: Axum (0.7+) — Tokio 官方亲儿子，天生具备多线程工作窃取异步调度能力
* **异步运行时**: Tokio — 支撑百万级高并发底座
* **数据库底层**: SQLx — 纯异步、编译期 SQL 合法性检查的数据库连接池
* **关系型数据库**: PostgreSQL (14+) — 承载复杂的区间冲突判定逻辑
* **演示前端**: HTML5 + Tailwind CSS (CDN) + 原生 JavaScript (Fetch API)

---

## 🚀 系统核心技术亮点

### 1. 微秒级高并发防超卖（悲观锁防线）

在售票核心事务中，系统在开启事务后第一时间向目标车次注入 `SELECT ... FOR UPDATE` 行级锁。在无锁竞态冲锋状态下，强行将并行的微秒级网络流量梳理为绝对安全的单列纵队，彻底终结超卖问题。

### 2. 区间段线段冲突触发器（防重叠占座）

不满足于传统的“总余票数减一”的简陋设计，系统将火车座位座位状态抽象为几何线段。在数据库内部利用触发器（`BEFORE INSERT ON tickets`）进行全表区间交集扫描：
当新订单的区间满足 $A < D \text{ AND } C < B$ 时，触发器直接拉响警报，抛出 `P0003` 异常，由上层转换为标准的 `409 Conflict` 网络响应。

### 3. 表间计数同步触发器（满足退票与自动修改）

为了死抠车票销售与退票时“自动修改相应车次剩余座位数”的指标，在数据库层架设了统一的 vanity 计数器触发器。无论是购票成功（`INSERT`）还是退票（`DELETE`），对应的 `trains.remaining_seats` 都会原子级无感同步增减。

### 4. 类型驱动的统一网络异常网（`IntoResponse`）

系统利用 Rust 强大的特征（Trait）系统，为自定义的 `SystemError` 枚举统一实现了 Axum 的 `IntoResponse` 特征。底层的 SQL 报错、冲突报错、参数非法报错，在网关最外层被编译期卡死，自动映射为符合互联网 RFC 标准的国际通用网络语言（如 400 Bad Request, 404 Not Found, 409 Conflict, 500 Internal Error），实现了全自动的依赖注入与零反射安全防御。

---

## 📊 课设硬性指标达成一览

系统不仅完成了所有标准增删改查，更下沉到数据库内核完成了高级 PL/pgSQL 脚本编写：

* [x] **实现车次管理**：通过 `POST /api/v1/trains` 接口持久化落盘。
* [x] **实现车次及价格管理（含到各站的价格）**：支持物理区段与价格关联配置。
* [x] **实现业务员管理**：完备的 Clerk 实体注册与生命周期管理。
* [x] **实现车票销售管理（触发器防超员、自动改座位）**：依靠 `trig_tickets_counter` 触发器实现。
* [x] **实现退票管理（触发器自动修改相应座位数）**：通过 `DELETE` 订单流水，触发器自动补回座位。
* [x] **创建存储过程 1**：统计指定车次指定发车时间的车票销售情况（存储过程：`proc_count_train_sales`）。
* [x] **创建存储过程 2**：统计指定日期各业务员车票的销售收入（存储过程：`proc_clerk_daily_revenue`）。
* [x] **创建表间关系**：建立严格的外键约束与级联清理。

---

## 💾 数据库核心脚本（SQL 筑基）

在启动项目前，请确保在 PostgreSQL 中执行了以下表结构、触发器与存储过程脚本：

```sql
-- 1. 基础建表与表间关系
CREATE TABLE trains (
    train_id INT PRIMARY KEY,
    name VARCHAR(50) NOT NULL,
    total_seats INT NOT NULL,
    remaining_seats INT NOT NULL
);

CREATE TABLE train_prices (
    price_id SERIAL PRIMARY KEY,
    train_id INT REFERENCES trains(train_id) ON DELETE CASCADE,
    from_station_id INT NOT NULL,
    to_station_id INT NOT NULL,
    price_cents INT NOT NULL,
    from_seq INT NOT NULL,
    to_seq INT NOT NULL
);

CREATE TABLE clerks (
    clerk_id INT PRIMARY KEY,
    name VARCHAR(50) NOT NULL,
    is_active BOOLEAN DEFAULT FALSE
);

CREATE TABLE tickets (
    ticket_id SERIAL PRIMARY KEY,
    train_id INT REFERENCES trains(train_id),
    clerk_id INT REFERENCES clerks(clerk_id),
    from_station_id INT NOT NULL,
    to_station_id INT NOT NULL,
    seat_number INT NOT NULL,
    price_cents INT NOT NULL,
    sold_at INT NOT NULL -- 实时 UNIX 时间戳
);

-- 2. 核心自动座位同步触发器 (Req 7)
CREATE OR REPLACE FUNCTION fn_modify_remaining_seats()
RETURNS TRIGGER AS $$
BEGIN
    IF (TG_OP = 'INSERT') THEN
        UPDATE trains SET remaining_seats = remaining_seats - 1 WHERE train_id = NEW.train_id;
        RETURN NEW;
    ELSIF (TG_OP = 'DELETE') THEN
        UPDATE trains SET remaining_seats = remaining_seats + 1 WHERE train_id = OLD.train_id;
        RETURN OLD;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trig_tickets_counter
AFTER INSERT OR DELETE ON tickets
FOR EACH ROW EXECUTE FUNCTION fn_modify_remaining_seats();

-- 3. 存储过程 1：统计指定车次/时间的销售情况 (Req 5)
CREATE OR REPLACE FUNCTION proc_count_train_sales(v_train_id INT, v_target_time INT)
RETURNS INT AS $$
DECLARE v_total_sold INT;
BEGIN
    SELECT COUNT(*)::INT INTO v_total_sold FROM tickets WHERE train_id = v_train_id AND sold_at = v_target_time;
    RETURN v_total_sold;
END;
$$ LANGUAGE plpgsql;

-- 4. 存储过程 2：统计指定日期各业务员的总收入 (Req 6)
CREATE OR REPLACE FUNCTION proc_clerk_daily_revenue(v_day_start INT)
RETURNS TABLE(out_clerk_id INT, out_total_revenue INT) AS $$
BEGIN
    RETURN QUERY
    SELECT clerk_id, SUM(price_cents)::INT FROM tickets
    WHERE sold_at >= v_day_start AND sold_at < (v_day_start + 86400)
    GROUP BY clerk_id;
END;
$$ LANGUAGE plpgsql;

```

---

## 🏃 部署与运行指南

### 1. 启动后端引擎

确保本地有可用的 PostgreSQL 实例（或通过 Docker 拉起），并拥有对应的数据库。

配置环境变量（或直接依赖代码内的默认配置回滚机制）：

```bash
export DATABASE_URL="postgres://postgres_admin:PasswordBase123!@127.0.0.1:5433/railway_db"

```

进入项目根目录，拉起多线程工作网：

```bash
cargo run --release

```

当终端亮起以下日志，代表网关就绪：

> 📡 铁路全栈系统合体成功！服务已安全挂载至 [http://127.0.0.1:8080](http://127.0.0.1:8080)
> 🔥 现在可以接受来自全球网络的 HTTP 并发流量冲击...

### 2. 启动沙盘可视化前端

1. 系统后端内置了宽松的跨域安全机制（`CorsLayer`），你不需要部署复杂的 Nginx 代理。
2. 直接在浏览器中双击打开根目录下的 `index.html`。
3. 点击页面中的 **“01 初始化系统基础数据”**，激活测试温床。
4. 点击红色的 **“02 100个请求并发全速冲锋！”**，即可在控制台和数据看板中实时观测：**1个成功，99个被悲观锁与触发器坚决拦截** 的工业级防御奇观。

---

## 📐 系统网络架构简图

```text
  [ 前端 HTML5 / 100并发并发冲锋 ]
               │
               ▼ (HTTP POST / JSON)
   [ Axum 多线程网关路由分发 ]
               │
               ▼ (Arc 状态机共享安全指针)
   [ DbStorage 悲观锁行锁定拦截 ] ──> 注入 SELECT ... FOR UPDATE
               │
               ▼ (进入事务控制链)
   [ Postgres 触发器与存储过程内核 ] ──> 执行几何交集冲突检测

```
