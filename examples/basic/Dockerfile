FROM rust:1.91-slim AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p basic-example

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/basic-example /usr/local/bin/app
ENV PORT=3000
EXPOSE 3000
CMD ["app"]
