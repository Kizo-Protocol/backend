# Build stage
FROM rustlang/rust:nightly as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests and create a dummy .env to prevent connection attempts
COPY Cargo.toml Cargo.lock* ./
RUN echo "DATABASE_URL=postgresql://user:pass@localhost/db" > .env

# Copy source code
COPY src ./src
COPY migrations ./migrations

# Copy .sqlx directory for offline SQLx compilation
COPY .sqlx ./.sqlx

# Build the application in release mode with offline mode for SQLx
ENV SQLX_OFFLINE=true
RUN cargo build --release --bin kizo-server

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libpq5 \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /app/target/release/kizo-server /usr/local/bin/kizo-server
COPY --from=builder /app/migrations ./migrations

# Set environment
ENV RUST_LOG=info

EXPOSE 3002

CMD ["kizo-server"]
