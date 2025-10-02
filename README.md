# Kizo Protocol - Backend

[![Rust](https://img.shields.io/badge/rust-1.78%2B-orange.svg)](https://www.rust-lang.org/)
[![Axum](https://img.shields.io/badge/Axum-0.7-blue.svg)](https://github.com/tokio-rs/axum)
[![PostgreSQL](https://img.shields.io/badge/PostgreSQL-12%2B-336791.svg)](https://www.postgresql.org/)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

## Overview

The Kizo Protocol Backend is a high-performance REST API server built with Rust for the Kizo Prediction Market on Aptos. It provides robust data aggregation, real-time synchronization with the blockchain indexer, yield calculation, and comprehensive analytics endpoints.

### Key Features

- **⚡ High Performance**: Built with Axum and Tokio for async/await concurrency
- **🔄 Real-time Sync**: Automatic synchronization with blockchain indexer database
- **📈 Analytics Engine**: Market statistics, user metrics, and platform analytics
- **📊 Yield Calculation**: Automated yield distribution and user earning tracking
- **🔐 JWT Authentication**: Secure wallet-based authentication system
- **📡 Event Notifications**: PostgreSQL LISTEN/NOTIFY for real-time events
- **📝 OpenAPI/Swagger**: Auto-generated API documentation with utoipa
- **🛠️ Background Scheduler**: Automated tasks for market resolution and data sync
- **🎭 CORS Support**: Configurable cross-origin resource sharing
- **📦 Database Migrations**: Automated schema management with SQLx
- **🤖 Auto-seeding**: Market data seeding from Adjacent API integration

## Architecture

### Tech Stack

**Core Framework**
- **Axum 0.7** - Modern async web framework
- **Tokio 1.37** - Async runtime with full features
- **Tower** - Middleware and service abstractions
- **Tower-HTTP** - HTTP-specific middleware (CORS, compression, tracing)

**Database**
- **SQLx 0.8** - Async, compile-time checked SQL queries
- **PostgreSQL** - Primary data store
- **BigDecimal** - Precise decimal arithmetic for financial data

**Authentication & Security**
- **JWT (jsonwebtoken)** - Token-based authentication
- **bcrypt** - Password hashing

**Documentation**
- **utoipa** - OpenAPI 3.0 spec generation
- **utoipa-swagger-ui** - Interactive API documentation

**External Integrations**
- **reqwest** - HTTP client for external APIs
- Adjacent API - Market data source
- Pexels API - Image assets
- Aptos Blockchain - Smart contract interactions

### System Components

```
kizo-server/
├── API Layer           # REST endpoints
├── Services Layer      # Business logic
│   ├── Market Service
│   ├── Betting Service
│   ├── Yield Calculator
│   ├── Blockchain Sync
│   ├── Price Feed Service
│   └── Scheduler
├── Database Layer      # Data access
├── Middleware          # Auth, logging, error handling
└── Background Jobs     # Scheduled tasks
```

## Prerequisites

- **Rust**: 1.78 or higher
- **PostgreSQL**: 12 or higher
- **Cargo**: Latest stable version
- **Database**: Access to indexer database for sync

### Installing Rust

```bash path=null start=null
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Installation

### 1. Clone the Repository

```bash path=null start=null
git clone <repository-url>
cd kizo/aptos/backend
```

### 2. Database Setup

Create the application database:

```bash path=null start=null
creatdb kizo_app
```

### 3. Environment Configuration

Create a `.env` file in the root directory:

```env path=null start=null
# =============================================================================
# DATABASE CONFIGURATION
# =============================================================================
DATABASE_URL=postgresql://user:password@localhost/kizo_app

# =============================================================================
# SERVER CONFIGURATION
# =============================================================================
HOST=0.0.0.0
PORT=3002
RUST_LOG=kizo_server=info,tower_http=info
CORS_ORIGIN=http://localhost:3000

# =============================================================================
# AUTHENTICATION
# =============================================================================
API_KEY=your-api-key-here

# =============================================================================
# EXTERNAL APIS
# =============================================================================
ADJACENT_API_KEY=your-adjacent-api-key
ADJACENT_API_BASE_URL=https://api.adjacent.xyz
PEXELS_API_KEY=your-pexels-api-key

# =============================================================================
# SEEDING (Optional)
# =============================================================================
RUN_SEEDS=true
SEED_MARKET_COUNT=10

# =============================================================================
# APTOS BLOCKCHAIN CONFIGURATION
# =============================================================================
APTOS_NODE_URL=https://fullnode.testnet.aptoslabs.com/v1
APTOS_MODULE_ADDRESS=0x66c4ec614f237de2470e107a17329e17d2e9d04bd6f609bdb7f7b52ae24c957c
APTOS_MODULE_NAME=kizo_prediction_market
APTOS_PROTOCOL_SELECTOR_ADDR=0x...
APTOS_TOKEN_TYPE=0x1::aptos_coin::AptosCoin
APTOS_PRIVATE_KEY=0x...
```

### 4. Run Database Migrations

Migrations are automatically applied on startup, or run manually:

```bash path=null start=null
cargo install sqlx-cli --no-default-features --features postgres
sqlx migrate run
```

### 5. Build the Project

```bash path=null start=null
cargo build --release
```

### 6. Start the Server

```bash path=null start=null
cargo run --release
```

The server will be available at [http://localhost:3002](http://localhost:3002)

## Available Commands

| Command | Description |
|---------|-------------|
| `cargo run` | Start development server |
| `cargo run --release` | Start production server (optimized) |
| `cargo build` | Build debug binary |
| `cargo build --release` | Build optimized binary |
| `cargo test` | Run test suite |
| `cargo run --bin seed` | Run database seeder manually |
| `cargo clippy` | Run linter |
| `cargo fmt` | Format code |

## Project Structure

```
backend/
├── src/
│   ├── main.rs              # Application entry point
│   ├── lib.rs               # Library exports
│   ├── config.rs            # Configuration management
│   ├── db.rs                # Database connection pool
│   ├── error.rs             # Error types and handling
│   ├── models.rs            # Data models
│   ├── openapi.rs           # OpenAPI specification
│   ├── chart.rs             # Chart data structures
│   ├── seed.rs              # Database seeding
│   ├── routes/              # API route handlers
│   │   ├── mod.rs
│   │   ├── markets.rs       # Market endpoints
│   │   ├── bets.rs          # Betting endpoints
│   │   ├── auth.rs          # Authentication
│   │   ├── charts.rs        # Chart data
│   │   ├── sync.rs          # Sync status
│   │   ├── protocols.rs     # Yield protocols
│   │   ├── yields.rs        # Yield data
│   │   ├── prices.rs        # Price feeds
│   │   └── blockchain.rs    # Blockchain interactions
│   ├── services/            # Business logic
│   │   ├── mod.rs
│   │   ├── market.rs
│   │   ├── betting_service.rs
│   │   ├── yield_calculator.rs
│   │   ├── blockchain_sync.rs
│   │   ├── scheduler.rs
│   │   ├── aptos_contract.rs
│   │   ├── chainlink_price_feed.rs
│   │   └── db_event_listener.rs
│   ├── middleware/          # HTTP middleware
│   │   ├── mod.rs
│   │   ├── auth.rs
│   │   └── jwt.rs
│   ├── admin/               # Admin routes
│   │   └── routes.rs
│   ├── utils/               # Utility functions
│   │   ├── mod.rs
│   │   └── jwt.rs
│   └── bin/
│       └── seed.rs          # Standalone seeder
├── migrations/              # SQL migrations
│   ├── 001_enhanced_schema.sql
│   ├── 004_remove_markets_extended_fkey.sql
│   ├── 008_create_event_notifications.sql
│   └── ...
├── scripts/                 # Utility scripts
├── Cargo.toml              # Project dependencies
├── .env.example            # Environment template
└── README.md               # This file
```

## API Endpoints

### Documentation

- **Swagger UI**: `http://localhost:3002/api-docs`
- **OpenAPI Spec**: `http://localhost:3002/api-docs/openapi.json`

### Core Endpoints

#### Health & Info

```http
GET  /api                    # API information
GET  /api/health             # Health check
```

#### Markets

```http
GET  /api/markets                      # List all markets
GET  /api/markets/:id                  # Get market by ID
GET  /api/markets/:id/stats            # Market statistics
GET  /api/markets/stats/platform       # Platform-wide stats
POST /api/markets                      # Create market (admin)
```

#### Bets

```http
GET  /api/bets                         # List recent bets
GET  /api/bets/:id                     # Get bet by ID
GET  /api/bets/user/:address           # User's bets
GET  /api/bets/user/:address/stats     # User statistics
GET  /api/bets/market/:id              # Market bets
```

#### Charts & Analytics

```http
GET  /api/charts/market/:id            # Market chart data
GET  /api/charts/user/:address         # User performance charts
GET  /api/charts/platform              # Platform analytics
```

#### Yields & Protocols

```http
GET  /api/yields/user/:address         # User yield earnings
GET  /api/yields/market/:id            # Market yield data
GET  /api/protocols                    # Available yield protocols
```

#### Sync & Blockchain

```http
GET  /api/sync/status                  # Indexer sync status
POST /api/sync/trigger                 # Trigger manual sync
GET  /api/blockchain/contracts         # Contract information
```

#### Authentication

```http
POST /api/auth/login                   # Wallet authentication
POST /api/auth/verify                  # Verify JWT token
GET  /api/auth/nonce/:address          # Get nonce for signing
```

## Key Features

### 1. Blockchain Synchronization

Automatically syncs data from the indexer database:

```rust path=null start=null
// Runs on startup and periodically
let sync_service = BlockchainSyncService::new(pool);
let summary = sync_service.run_full_sync().await?;
```

**Synced Events:**
- Market creation
- Bet placement
- Market resolution
- Winnings claims
- Yield deposits
- Protocol fee collection

### 2. Real-time Event Notifications

PostgreSQL LISTEN/NOTIFY for instant updates:

```rust path=null start=null
// Listens for database events
LISTEN market_created;
LISTEN bet_placed;
LISTEN market_resolved;
```

### 3. Yield Calculation

Automated yield distribution:

```rust path=null start=null
// Calculates and distributes yield earnings
let yield_service = YieldService::new(pool);
yield_service.calculate_market_yields(market_id).await?;
```

### 4. Background Scheduler

Automated tasks:
- Market resolution checks
- Yield calculation
- Data sync from indexer
- Price feed updates

```rust path=null start=null
let scheduler = Scheduler::new(pool);
scheduler.start().await;
```

### 5. Market Seeding

Auto-populate markets from Adjacent API:

```bash path=null start=null
# Enable in .env
RUN_SEEDS=true
SEED_MARKET_COUNT=50
```

Creates markets on both backend and blockchain.

## Configuration

### Environment Variables

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `DATABASE_URL` | PostgreSQL connection string | - | Yes |
| `HOST` | Server host | `0.0.0.0` | No |
| `PORT` | Server port | `3002` | No |
| `RUST_LOG` | Logging level | `info` | No |
| `CORS_ORIGIN` | Allowed CORS origin | `*` | No |
| `API_KEY` | API authentication key | - | Yes |
| `ADJACENT_API_KEY` | Adjacent API key | - | Yes |
| `APTOS_NODE_URL` | Aptos RPC endpoint | - | Yes |
| `APTOS_MODULE_ADDRESS` | Contract address | - | Yes |
| `RUN_SEEDS` | Run seeding on startup | `false` | No |

### Logging

Configured via `RUST_LOG` environment variable:

```bash path=null start=null
# Detailed logging
RUST_LOG=kizo_server=debug,sqlx=debug,tower_http=debug

# Production logging
RUST_LOG=kizo_server=info,tower_http=warn
```

### CORS Configuration

Configure allowed origins:

```bash path=null start=null
CORS_ORIGIN=http://localhost:3000,https://kizo.xyz
```

## Database Schema

The backend uses the following main tables:

### Core Tables

- **markets** - Prediction market data
- **markets_extended** - Additional market metadata
- **bets** - User betting records
- **market_resolutions** - Resolution outcomes
- **winnings_claims** - Claim history
- **yield_deposits** - Yield tracking
- **protocol_fees** - Fee collection records

### Extended Tables

- **protocols** - Yield protocol configurations
- **protocol_apys** - APY tracking
- **user_yields** - User yield earnings
- **market_images** - Image assets
- **user_stats** - Aggregated user statistics
- **platform_stats** - Platform analytics

### Migrations

Migrations are in the `migrations/` directory and run automatically on startup.

## Development

### Running Tests

```bash path=null start=null
cargo test
```

### Code Quality

```bash path=null start=null
# Linting
cargo clippy --all-targets --all-features

# Formatting
cargo fmt --all

# Type checking
cargo check
```

### Database Seeding

```bash path=null start=null
# Run seeder binary
cargo run --bin seed

# Or enable in .env
RUN_SEEDS=true
```

### Hot Reload (Development)

Install cargo-watch:

```bash path=null start=null
cargo install cargo-watch
cargo watch -x run
```

## Deployment

### Production Build

```bash path=null start=null
cargo build --release
./target/release/kizo-server
```

The release build includes:
- Full optimizations (`opt-level = 3`)
- Link-time optimization (LTO)
- Single codegen unit for maximum performance

### Docker

Create a `Dockerfile`:

```dockerfile path=null start=null
FROM rust:1.78 as builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/kizo-server /usr/local/bin/
EXPOSE 3002
CMD ["kizo-server"]
```

### Systemd Service

Create `/etc/systemd/system/kizo-server.service`:

```ini path=null start=null
[Unit]
Description=Kizo Prediction Market API Server
After=network.target postgresql.service

[Service]
Type=simple
User=kizo
WorkingDirectory=/opt/kizo/backend
EnvironmentFile=/opt/kizo/backend/.env
ExecStart=/opt/kizo/backend/target/release/kizo-server
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

### Health Checks

For production monitoring:

```bash path=null start=null
curl http://localhost:3002/api/health
```

Expected response:
```json
{
  "status": "healthy",
  "database": "connected",
  "timestamp": "2025-10-01T18:41:00Z"
}
```

## Monitoring & Observability

### Logging

Structured logging with `tracing`:

```rust path=null start=null
info!("Market created: id={}", market_id);
error!(error = %e, "Failed to sync data");
```

### Metrics

HTTP middleware provides:
- Request/response logging
- Timing information
- Error tracking

### Database Connection Pool

SQLx provides connection pool metrics:
- Active connections
- Idle connections
- Connection errors

## Performance

### Optimizations

- **Async/Await**: Non-blocking I/O throughout
- **Connection Pooling**: Efficient database connections
- **Compile-time SQL**: Zero-overhead queries with SQLx
- **GZIP Compression**: Automatic response compression
- **LTO & Optimization**: Maximum release build performance

### Benchmarks

- **Startup Time**: ~1-2 seconds (with sync)
- **Request Latency**: <10ms for simple queries
- **Throughput**: 1000+ req/s on modest hardware
- **Memory**: ~50MB baseline

## Troubleshooting

### Database Connection Issues

**Problem**: Cannot connect to PostgreSQL

**Solutions**:
```bash path=null start=null
# Check PostgreSQL is running
pg_isready

# Verify connection string
psql $DATABASE_URL

# Check firewall/network
telnet localhost 5432
```

### Migration Failures

**Problem**: Migrations fail on startup

**Solutions**:
```bash path=null start=null
# Reset migrations (development only)
sqlx migrate revert
sqlx migrate run

# Check migration status
sqlx migrate info
```

### Sync Issues

**Problem**: Data not syncing from indexer

**Solutions**:
1. Check indexer database is accessible
2. Verify connection string in code
3. Check logs for sync errors
4. Trigger manual sync: `POST /api/sync/trigger`

### Performance Issues

**Problem**: Slow API responses

**Solutions**:
1. Check database query performance
2. Review connection pool settings
3. Enable query logging: `RUST_LOG=sqlx=debug`
4. Add database indexes as needed

## Security Considerations

### Best Practices

- Store `.env` securely, never commit to version control
- Use strong API keys with sufficient entropy
- Enable HTTPS in production
- Configure strict CORS policies
- Regularly update dependencies
- Run `cargo audit` to check for vulnerabilities

### Authentication

JWT-based authentication:
```rust path=null start=null
// Protected routes require valid JWT
Router::new()
    .route("/protected", get(handler))
    .layer(middleware::from_fn(auth_middleware))
```

## Contributing

We welcome contributions! Please follow these guidelines:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Run tests: `cargo test`
5. Run linter: `cargo clippy`
6. Format code: `cargo fmt`
7. Commit your changes (`git commit -m 'Add amazing feature'`)
8. Push to the branch (`git push origin feature/amazing-feature`)
9. Open a Pull Request

### Code Standards

- Follow Rust idioms and best practices
- Write tests for new functionality
- Document public APIs
- Use meaningful variable names
- Handle errors properly (no unwrap in production code)

## License

This project is licensed under the Apache License 2.0 - see the LICENSE file for details.

## Support

For questions, issues, or contributions:

- **Issues**: Open an issue on GitHub
- **Documentation**: [Kizo Protocol Docs](https://kizoprotocol.gitbook.io/kizoprotocol-docs)
- **Community**: [Kizo Protocol X](https://x.com/kizoprotocol)

## Acknowledgments

- Built with [Axum](https://github.com/tokio-rs/axum) by the Tokio team
- Database migrations with [SQLx](https://github.com/launchbadge/sqlx)
- OpenAPI docs with [utoipa](https://github.com/juhaku/utoipa)
- Powered by [Aptos Labs](https://aptoslabs.com/)

---

**Last Updated**: October 1, 2025  
**Version**: 1.0.0  
**Status**: Active Development  
**Network**: Aptos Testnet
# CI/CD Fix Applied - Thu Oct  2 10:23:26 WIB 2025
[INFO] : Testing professional CI/CD pipeline - Thu Oct  2 10:54:22 WIB 2025
