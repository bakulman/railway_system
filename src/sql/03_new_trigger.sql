-- 1. 废除旧的计件触发器
DROP TRIGGER IF EXISTS trig_ticket_sold ON tickets;

-- 2. 创造全新的区间冲突判定函数
CREATE OR REPLACE FUNCTION fn_check_segment_conflict()
RETURNS TRIGGER AS $$
DECLARE
    v_from_seq INT;
    v_to_seq INT;
BEGIN
    -- 先找出当前请求的始发站和终点站的物理顺序号(seq)
    SELECT from_seq, to_seq INTO v_from_seq, v_to_seq
    FROM train_prices
    WHERE train_id = NEW.train_id
      AND from_station_id = NEW.from_station_id
      AND to_station_id = NEW.to_station_id;

    -- 业务防御：如果没有配置这段区间的票价，说明路线非法
    IF v_from_seq IS NULL THEN
        RAISE EXCEPTION '非法路线配置' USING ERRCODE = 'P0002';
    END IF;

    -- 核心算法：去全表扫描，看看有没有同车次、同座位的区间重叠车票
    -- 利用公式：A < D AND C < B
    IF EXISTS (
        SELECT 1 FROM tickets t
        JOIN train_prices tp
          ON t.train_id = tp.train_id
         AND t.from_station_id = tp.from_station_id
         AND t.to_station_id = tp.to_station_id
        WHERE t.train_id = NEW.train_id
          AND t.seat_number = NEW.seat_number
          AND v_from_seq < tp.to_seq  -- 新起点的序号 < 老终点的序号
          AND tp.from_seq < v_to_seq  -- 老起点的序号 < 新终点的序号
    ) THEN
        -- 暗号 P0003：代表座位区间冲突，已被占用！
        RAISE EXCEPTION '座位 % 在该区间已被锁定，无法购买', NEW.seat_number
        USING ERRCODE = 'P0003';
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 3. 重新死死绑定到 tickets 表上
CREATE TRIGGER trig_ticket_segment_check
BEFORE INSERT ON tickets
FOR EACH ROW
EXECUTE FUNCTION fn_check_segment_conflict();
