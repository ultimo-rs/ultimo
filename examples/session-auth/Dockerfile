FROM rust:1.86-slim AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY ultimo/ ultimo/
COPY examples/session-auth/ examples/session-auth/
RUN cargo build --release -p session-auth-example

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/session-auth-example /usr/local/bin/app
ENV PORT=3000
EXPOSE 3000
CMD ["app"]
