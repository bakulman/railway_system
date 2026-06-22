-- 1. 重新创建/覆盖控座函数
CREATE OR REPLACE FUNCTION fn_on_ticket_sold()
RETURNS TRIGGER AS $$
BEGIN
    -- 强制扣减余票
    UPDATE trains
    SET remaining_seats = remaining_seats - 1
    WHERE train_id = NEW.train_id;

    -- 绝杀：如果发现扣成负数了，立刻抛出 P0001 异常，强行中断并回滚事务！
    IF (SELECT remaining_seats FROM trains WHERE train_id = NEW.train_id) < 0 THEN
        RAISE EXCEPTION '火车座位已售罄，无法出票！' USING ERRCODE = 'P0001';
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 2. 强行先删后建触发器，确保它绝对绑在 tickets 表上
DROP TRIGGER IF EXISTS trig_ticket_sold ON tickets;

CREATE TRIGGER trig_ticket_sold
AFTER INSERT ON tickets
FOR EACH ROW
EXECUTE FUNCTION fn_on_ticket_sold();
