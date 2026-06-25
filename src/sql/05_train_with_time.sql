-- 修正后的 05_train_with_time.sql
CREATE OR REPLACE FUNCTION proc_count_train_sales(v_train_id INT, v_target_time BIGINT)
RETURNS INT AS $$
DECLARE
    v_total_sold INT;
BEGIN
    SELECT COUNT(*)::INT INTO v_total_sold
    FROM tickets
    WHERE train_id = v_train_id
      AND sold_at = v_target_time;

    RETURN v_total_sold;
END;
$$ LANGUAGE plpgsql;
