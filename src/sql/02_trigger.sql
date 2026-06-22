CREATE OR REPLACE FUNCTION fn_on_ticket_sold()
RETURNS TRIGGER AS $$
BEGIN
    -- 1. 核心防御：更新对应的火车，让剩余座位减 1
    UPDATE trains
    SET remaining_seats = remaining_seats - 1
    WHERE train_id = NEW.train_id;

    -- 2. 核心检查：如果发现减完之后座位直接变成了负数，说明票卖超了！
    IF (SELECT remaining_seats FROM trains WHERE train_id = NEW.train_id) < 0 THEN
        -- 扔出自定义异常
        RAISE EXCEPTION '火车 ID % 的座位已经售罄，无法继续出票！', NEW.train_id
        USING ERRCODE = 'P0001';
    END IF;

    -- 一切正常，允许插入
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
