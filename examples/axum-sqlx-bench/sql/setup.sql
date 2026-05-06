-- TechEmpower-spec schema: World (10000 rows) + Fortune (12 rows incl. JP UTF-8).
-- Mirrors toolset/databases/postgres/create-postgres.sql in the TFB repo.

DROP TABLE IF EXISTS world;
CREATE TABLE world (
    id integer PRIMARY KEY,
    randomnumber integer NOT NULL DEFAULT 0
);

INSERT INTO world (id, randomnumber)
SELECT i, LEAST(FLOOR(random() * 10000 + 1)::int, 10000)
FROM generate_series(1, 10000) AS i;

DROP TABLE IF EXISTS fortune;
CREATE TABLE fortune (
    id integer PRIMARY KEY,
    message varchar(2048) NOT NULL
);

INSERT INTO fortune (id, message) VALUES
    (1,  'fortune: No such file or directory'),
    (2,  'A computer scientist is someone who fixes things that aren''t broken.'),
    (3,  'After enough decimal places, nobody gives a damn.'),
    (4,  'A bad random number generator: 1, 1, 1, 1, 1, 4.33e+67, 1, 1, 1'),
    (5,  'A computer program does what you tell it to do, not what you want it to do.'),
    (6,  'Emacs is a nice operating system, but I prefer UNIX. — Tom Christaensen'),
    (7,  'Any program that runs right is obsolete.'),
    (8,  'A list is only as strong as its weakest link. — Donald Knuth'),
    (9,  'Feature: A bug with seniority.'),
    (10, 'Computers make very fast, very accurate mistakes.'),
    (11, '<script>alert("This should not be displayed in a browser alert box.");</script>'),
    (12, 'フレームワークのベンチマーク');

ANALYZE world;
ANALYZE fortune;
