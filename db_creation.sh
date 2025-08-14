#!/usr/bin/env bash
set -e

psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" <<-EOSQL

    CREATE DATABASE "rrate-limiter" WITH OWNER ${POSTGRES_USER};
    GRANT ALL PRIVILEGES ON DATABASE "rrate-limiter" TO ${POSTGRES_USER};

    \connect "rrate-limiter"
    \i /db.sql

	
EOSQL