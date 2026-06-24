CREATE OR REPLACE FUNCTION proc_clerk_daily_revenue(v_day_start INT)
RETURNS TABLE(out_clerk_id INT, out_total_revenue INT) AS $$
BEGIN
    RETURN QUERY
    SELECT clerk_id, SUM(price_cents)::INT
    FROM tickets
    WHERE sold_at >= v_day_start
      AND sold_at < (v_day_start + 86400) -- 加上一天的秒数（24小时）, 即为当天截止
    GROUP BY clerk_id;
END;
$$ LANGUAGE plpgsql;
