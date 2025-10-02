# Build stage with database setup
FROM rustlang/rust:nightly AS builder

WORKDIR /app

# Install build dependencies including PostgreSQL and sudo
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    postgresql \
    postgresql-contrib \
    sudo \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests first to cache dependencies
COPY Cargo.toml Cargo.lock* ./

# Copy source code and migrations
COPY src ./src
COPY migrations ./migrations

# Start PostgreSQL and set up database for SQLx
RUN service postgresql start && \
    su -c "createdb kizo_build" postgres && \
    su -c "psql -c \"CREATE USER kizo_user WITH PASSWORD 'kizo_pass';\"" postgres && \
    su -c "psql -c \"GRANT ALL PRIVILEGES ON DATABASE kizo_build TO kizo_user;\"" postgres && \
    su -c "psql -c \"GRANT ALL ON SCHEMA public TO kizo_user;\"" postgres && \
    su -c "psql -c \"GRANT CREATE ON SCHEMA public TO kizo_user;\"" postgres && \
    export DATABASE_URL="postgresql://kizo_user:kizo_pass@localhost/kizo_build" && \
    echo "DATABASE_URL=postgresql://kizo_user:kizo_pass@localhost/kizo_build" > .env && \
    cargo install sqlx-cli --no-default-features --features rustls,postgres && \
    sqlx migrate run && \
    cargo build --release --bin kizo-server && \
    service postgresql stop

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
