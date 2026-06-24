CREATE OR REPLACE FUNCTION proc_count_train_sales(v_train_id INT, v_target_time INT)
RETURNS INT AS $$
DECLARE
    v_total_sold INT;
BEGIN
    SELECT COUNT(*)::INT INTO v_total_sold
    FROM tickets
    WHERE train_id = v_train_id
      AND sold_at = v_target_time; -- 在我们的模型中用整数时间戳代表发车/售出时间

    RETURN v_total_sold;
END;
$$ LANGUAGE plpgsql;
