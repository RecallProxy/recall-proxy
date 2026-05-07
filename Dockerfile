# Build stage
FROM rust:1-bookworm AS builder

WORKDIR /usr/src/recall-proxy
COPY . .

# Build the gateway binary
RUN cargo build --release -p recall-proxy-gateway

# Final stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/local/bin

COPY --from=builder /usr/src/recall-proxy/target/release/recall-proxy-gateway .

# Default runtime configuration
ENV RECALL_PROXY_BIND_ADDRESS=0.0.0.0:8080

EXPOSE 8080

CMD ["./recall-proxy-gateway"]
