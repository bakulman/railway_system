-- 修正后的 07_delete_all.sql
CREATE OR REPLACE FUNCTION dall()
RETURNS void AS $$
BEGIN
    -- TRUNCATE 会瞬间清空表结构，RESTART IDENTITY 会重置所有的自增 ID (如 ticket_id)
    -- CASCADE 会自动级联清空有关联外键的表
    TRUNCATE TABLE stations, trains, clerks, train_prices, tickets RESTART IDENTITY CASCADE;
END;
$$ LANGUAGE plpgsql;
