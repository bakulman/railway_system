
-- 1. 创建业务员表
CREATE TABLE clerks (
    clerk_id INT PRIMARY KEY,
    name VARCHAR(50) NOT NULL UNIQUE,
    is_active BOOLEAN NOT NULL DEFAULT TRUE
);

-- 2. 创建车次主表
CREATE TABLE trains (
    train_id INT PRIMARY KEY,
    name VARCHAR(20) NOT NULL UNIQUE,
    total_seats INT NOT NULL,
    remaining_seats INT NOT NULL
);

-- 1. 必须最先创建车站基础表
CREATE TABLE stations (
    station_id INT PRIMARY KEY,
    station_name VARCHAR(50) NOT NULL UNIQUE
);

-- 2. 修改车次价格表，追加外键
CREATE TABLE train_prices (
    train_id INT,
    from_station_id INT REFERENCES stations(station_id), -- 👈 强绑定
    to_station_id INT REFERENCES stations(station_id),   -- 👈 强绑定
    price_cents INT NOT NULL,
    from_seq INT NOT NULL,
    to_seq INT NOT NULL,
    PRIMARY KEY (train_id, from_station_id, to_station_id),
    FOREIGN KEY (train_id) REFERENCES trains(train_id) ON DELETE CASCADE
);

-- 3. 修改车票流水表，追加外键
CREATE TABLE tickets (
    ticket_id SERIAL PRIMARY KEY,
    train_id INT NOT NULL REFERENCES trains(train_id),
    clerk_id INT NOT NULL REFERENCES clerks(clerk_id),
    from_station_id INT NOT NULL REFERENCES stations(station_id), -- 👈 强绑定
    to_station_id INT NOT NULL REFERENCES stations(station_id),   -- 👈 强绑定
    seat_number INT NOT NULL,
    price_cents INT NOT NULL,
    sold_at BIGINT NOT NULL
);


-- 2. 为 tickets 表追加复合索引（专为 03 号区间防冲突触发器优化）
-- 这样每次查这个座位的区间占用时，能达到极速的 Index Seek
CREATE INDEX idx_tickets_train_seat ON tickets (train_id, seat_number);

-- 3. 为 tickets 表的 sold_at 追加索引（专为 05、06 号存储过程优化）
-- 如果业务会频繁按天查账，这个索引不可或缺
CREATE INDEX idx_tickets_sold_at ON tickets (sold_at);
