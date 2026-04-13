-- Integration test schema for Sentinel ORM
-- Run: psql $DATABASE_URL -f tests/integration/setup.sql

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Drop existing tables (idempotent re-runs)
DROP TABLE IF EXISTS posts CASCADE;
DROP TABLE IF EXISTS users CASCADE;
DROP TABLE IF EXISTS type_roundtrip CASCADE;

-- Standard model for CRUD tests
CREATE TABLE users (
    id          SERIAL PRIMARY KEY,
    name        TEXT NOT NULL,
    email       TEXT NOT NULL UNIQUE,
    active      BOOLEAN NOT NULL DEFAULT true,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Relation target for Phase 4
CREATE TABLE posts (
    id          SERIAL PRIMARY KEY,
    user_id     INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title       TEXT NOT NULL,
    body        TEXT NOT NULL DEFAULT '',
    published   BOOLEAN NOT NULL DEFAULT false,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_posts_user_id ON posts(user_id);

-- Every Value variant for roundtrip testing
CREATE TABLE type_roundtrip (
    id              SERIAL PRIMARY KEY,
    -- existing
    bool_col        BOOLEAN,
    int_col         INTEGER,
    bigint_col      BIGINT,
    double_col      DOUBLE PRECISION,
    text_col        TEXT,
    uuid_col        UUID,
    timestamptz_col TIMESTAMPTZ,
    bytea_col       BYTEA,
    -- new scalars
    smallint_col    SMALLINT,
    float_col       REAL,
    json_col        JSON,
    jsonb_col       JSONB,
    numeric_col     NUMERIC(20,6),
    money_col       MONEY,
    xml_col         XML,
    bit_col         BIT VARYING(64),
    -- temporal
    date_col        DATE,
    time_col        TIME,
    timestamp_col   TIMESTAMP,
    -- network
    inet_col        INET,
    cidr_col        CIDR,
    macaddr_col     MACADDR,
    -- interval
    interval_col    INTERVAL,
    -- geometric
    point_col       POINT,
    line_col        LINE,
    lseg_col        LSEG,
    box_col         BOX,
    circle_col      CIRCLE,
    -- ranges
    int4range_col   INT4RANGE,
    int8range_col   INT8RANGE,
    numrange_col    NUMRANGE,
    tsrange_col     TSRANGE,
    tstzrange_col   TSTZRANGE,
    daterange_col   DATERANGE,
    -- arrays
    int_array_col   INTEGER[],
    text_array_col  TEXT[]
);
