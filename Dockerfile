FROM rust:1.90 AS builder
WORKDIR /app
COPY . .
ENV SQLX_OFFLINE=true
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libpq5 ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/ferroque /usr/local/bin/ferroque
CMD ["ferroque"]