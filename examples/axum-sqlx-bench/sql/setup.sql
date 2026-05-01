-- Same schema as axum-bench/sql/setup.sql for fair comparison.
DROP TABLE IF EXISTS world;
CREATE TABLE world (
    id integer PRIMARY KEY,
    randomnumber integer NOT NULL
);

INSERT INTO world (id, randomnumber)
SELECT i, (random() * 9999 + 1)::int
FROM generate_series(1, 10000) AS i;

ANALYZE world;
