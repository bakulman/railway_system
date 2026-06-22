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

-- 3. 创建车次站点与票价表（复合主键 + 级联删除外键）
CREATE TABLE train_prices (
    train_id INT,
    from_station_id INT NOT NULL,
    to_station_id INT NOT NULL,
    price_cents INT NOT NULL,
    from_seq INT NOT NULL,
    to_seq INT NOT NULL,
    PRIMARY KEY (train_id, from_station_id, to_station_id),
    FOREIGN KEY (train_id) REFERENCES trains(train_id) ON DELETE CASCADE
);

-- 4. 创建车票流水表（自增主键 + 严格外键约束）
CREATE TABLE tickets (
    ticket_id SERIAL PRIMARY KEY, -- 必须显式声明主键
    train_id INT NOT NULL,
    clerk_id INT NOT NULL,
    from_station_id INT NOT NULL,
    to_station_id INT NOT NULL,
    seat_number INT NOT NULL,
    price_cents INT NOT NULL,
    sold_at BIGINT NOT NULL,
    FOREIGN KEY (train_id) REFERENCES trains(train_id),
    FOREIGN KEY (clerk_id) REFERENCES clerks(clerk_id)
);
