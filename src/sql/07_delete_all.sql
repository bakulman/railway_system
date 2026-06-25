CREATE OR REPLACE FUNCTION dall()
RETURNS void AS $$
BEGIN
    DELETE FROM tickets;
    DELETE FROM trains;
    DELETE FROM clerks;
    DELETE FROM train_prices;
END;
$$ LANGUAGE plpgsql;
