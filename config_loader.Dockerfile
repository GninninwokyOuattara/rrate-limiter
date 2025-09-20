ARG RUST_VERSION=1.89.0
ARG APP_NAME=config-loader

FROM rust:${RUST_VERSION}-alpine AS build
ARG APP_NAME
WORKDIR /app

RUN apk add --no-cache clang lld musl-dev git


RUN --mount=type=bind,source=rrl_core,target=rrl_core \
    --mount=type=bind,source=config_loader/src,target=config_loader/src \
    --mount=type=bind,source=config_loader/Cargo.toml,target=config_loader/Cargo.toml \
    --mount=type=bind,source=config_loader/Cargo.lock,target=config_loader/Cargo.lock \
    --mount=type=cache,target=/root/.gem \
    --mount=type=cache,target=/app/config_loader/target/ \
    --mount=type=cache,target=/usr/local/cargo/git/db \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    cd config_loader && \
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
