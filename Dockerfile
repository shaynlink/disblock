# syntax=docker/dockerfile:1
FROM rust:1.98-slim-bookworm AS build
WORKDIR /app
# Queries are checked against .sqlx (cargo sqlx prepare --workspace), not a live database.
ENV SQLX_OFFLINE=true
COPY Cargo.toml Cargo.lock ./
COPY crates crates
COPY migrations migrations
COPY .sqlx .sqlx
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --locked -p disblock-server \
    && cp target/release/disblock-server /usr/local/bin/disblock-server

FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=build /usr/local/bin/disblock-server /disblock-server
ENV PORT=8080
EXPOSE 8080
ENTRYPOINT ["/disblock-server"]
