FROM rust:1.86-slim AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p openapi-demo --bin rest-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/rest-server /usr/local/bin/app
COPY examples/openapi-demo/swagger-ui.html /app/swagger-ui.html
WORKDIR /app
ENV PORT=3000
EXPOSE 3000
CMD ["app"]
