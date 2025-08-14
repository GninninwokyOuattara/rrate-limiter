FROM postgres:alpine
COPY ./db_creation.sh /docker-entrypoint-initdb.d/
COPY ./db.sql /db.sql

