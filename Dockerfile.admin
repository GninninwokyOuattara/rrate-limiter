# syntax=docker/dockerfile:1

ARG RUST_VERSION=1.89.0
ARG APP_NAME=admin


FROM rust:${RUST_VERSION}-alpine AS build
ARG APP_NAME
WORKDIR /app

RUN apk add --no-cache clang lld musl-dev git

RUN --mount=type=bind,source=rrl_core,target=rrl_core \
    --mount=type=cache,target=/app/rrl_core_admin/target/ \
    --mount=type=bind,source=admin/src,target=admin/src \
    --mount=type=bind,source=admin/Cargo.toml,target=admin/Cargo.toml \
    --mount=type=bind,source=admin/Cargo.lock,target=admin/Cargo.lock \
    --mount=type=cache,target=/app/admin/target/ \
    --mount=type=cache,target=/root/.gem \
    --mount=type=cache,target=/usr/local/cargo/git/db \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    cd admin && \
    cargo build --locked --release && \
    cp ./target/release/$APP_NAME /bin/server


FROM alpine:3.18 AS final


ARG UID=10001
RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    appuser
USER appuser

COPY --from=build /bin/server /bin/

EXPOSE 3000

CMD ["/bin/server"]
