CREATE OR REPLACE FUNCTION proc_clerk_daily_revenue(v_day_start BIGINT)
RETURNS TABLE(out_clerk_id INT, out_total_revenue BIGINT) AS $$
BEGIN
    RETURN QUERY
    SELECT clerk_id, SUM(price_cents)::BIGINT
    FROM tickets
    WHERE sold_at >= v_day_start
      AND sold_at < (v_day_start + 86400)
    GROUP BY clerk_id;
END;
$$ LANGUAGE plpgsql;
