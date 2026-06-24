-- 创建一个统一管理剩余座位增减的函数
CREATE OR REPLACE FUNCTION fn_modify_remaining_seats()
RETURNS TRIGGER AS $$
BEGIN
    IF (TG_OP = 'INSERT') THEN
        -- 当有人买票成功（越过了区间触发器防线后）
        UPDATE trains
        SET remaining_seats = remaining_seats - 1
        WHERE train_id = NEW.train_id;
        RETURN NEW;

    ELSIF (TG_OP = 'DELETE') THEN
        -- 当有人退票时, 自动把车的剩余总座位加回来！
        UPDATE trains
        SET remaining_seats = remaining_seats + 1
        WHERE train_id = OLD.train_id;
        RETURN OLD;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

-- 绑定到 tickets 表上（在动作执行完之后修改计数器）
DROP TRIGGER IF EXISTS trig_tickets_counter ON tickets;
CREATE TRIGGER trig_tickets_counter
AFTER INSERT OR DELETE ON tickets
FOR EACH ROW
EXECUTE FUNCTION fn_modify_remaining_seats();
