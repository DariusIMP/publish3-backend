# Publish3 Backend

A Rust-based backend API for a publishing platform, built with Actix-web, PostgreSQL, Redis, and MinIO/S3 storage. Features authentication via Privy and comprehensive API endpoints for managing publications, authors, citations, and users.

Check https://github.com/DariusIMP/Publish3 for the main repository.

## Features

- **RESTful API** with Actix-web framework
- **PostgreSQL** database with SQLx for async database operations
- **Redis** for caching and session management
- **MinIO/S3** for file storage
- **Privy** authentication and authorization
- **Docker Compose** for local development
- **CORS** enabled for frontend integration
- **Comprehensive logging** with tracing

## Prerequisites

Before you begin, ensure you have the following installed:

- [Rust](https://www.rust-lang.org/tools/install) (latest stable version)
- [Docker](https://docs.docker.com/get-docker/) and [Docker Compose](https://docs.docker.com/compose/install/)
- [Git](https://git-scm.com/downloads)

## Quick Start

### 1. Clone the Repository

```bash
git clone https://github.com/DariusIMP/publish3-backend.git
cd publish3-backend
```

### 2. Set Up Environment Variables

Create a `.env` file in the project root:

```bash
cp .env.example .env  # If you have an example file
# Otherwise, create .env manually
```

Edit the `.env` file with your configuration:

```env
# Database
DATABASE_URL=postgres://postgres:postgres@localhost:6500/publish3
REDIS_URL=redis://localhost:6379

# Server Configuration
SERVER_ADDRESS=0.0.0.0
SERVER_PORT=8080
SERVER_BASE_URL=http://localhost:8080
CLIENT_ORIGIN=http://localhost:3000  # Your frontend URL

# S3/MinIO Configuration
S3_ACCESS_KEY=minioadmin
S3_SECRET_KEY=minioadmin
S3_ENDPOINT=http://localhost:9000

# Privy Authentication
PRIVY_APP_ID=your_privy_app_id
PRIVY_APP_SECRET=your_privy_app_secret
PRIVY_JWT_VERIFICATION_KEY=your_base64_encoded_jwt_verification_key

# Docker Services (used in docker-compose.yml)
POSTGRES_USER=postgres
POSTGRES_PASSWORD=postgres
POSTGRES_DB=publish3
PGADMIN_DEFAULT_EMAIL=admin@admin.com
PGADMIN_DEFAULT_PASSWORD=admin
MINIO_ROOT_USER=minioadmin
MINIO_ROOT_PASSWORD=minioadmin
MINIO_SERVER_URL=http://localhost:9000
```

### 3. Start Docker Services

Start the required services using Docker Compose:

```bash
docker-compose up -d
```

This will start:
- **PostgreSQL** on port 6500
- **pgAdmin** on port 5050 (http://localhost:5050)
- **Redis** on port 6379
- **MinIO** on ports 9000 and 9001 (http://localhost:9001 for console)

### 4. Run Database Migrations

Apply database migrations:

```bash
# Install sqlx-cli if not already installed
cargo install sqlx-cli

# Run migrations
sqlx migrate run
```

### 5. Build and Run the Application

#### Development Mode

```bash
cargo run
```

The server will start at `http://localhost:8080`.

#### Production Build

```bash
cargo build --release
./target/release/publish3-backend
```

### 6. Verify Installation

Check if the server is running:

```bash
curl http://localhost:8080
```

You should receive a response or see the server logs indicating it's running.

## Project Structure

```
publish3-backend/
├── src/
│   ├── main.rs              # Application entry point
│   ├── config.rs            # Configuration management
│   ├── api/                 # API endpoints
│   │   ├── publications/    # Publication endpoints
│   │   ├── authors/         # Author endpoints
│   │   ├── citations/       # Citation endpoints
│   │   ├── users/           # User endpoints
│   │   └── mod.rs          # API configuration
│   ├── auth/                # Authentication module
│   ├── db/                  # Database layer
│   │   ├── sql/            # PostgreSQL operations
│   │   └── s3/             # S3/MinIO operations
│   ├── common/             # Common utilities
│   └── lib.rs              # Library exports
├── migrations/             # Database migrations
├── docker-compose.yml      # Docker services configuration
├── Cargo.toml             # Rust dependencies
└── .env                   # Environment variables
```

## API Endpoints

### Publications
- `GET /api/publications` - List all publications
- `GET /api/publications/{id}` - Get publication by ID
- `POST /api/publications` - Create new publication
- `PUT /api/publications/{id}` - Update publication
- `DELETE /api/publications/{id}` - Delete publication

### Authors
- `GET /api/authors` - List all authors
- `GET /api/authors/{id}` - Get author by ID
- `POST /api/authors` - Create new author
- `PUT /api/authors/{id}` - Update author
- `DELETE /api/authors/{id}` - Delete author

### Citations
- `GET /api/citations` - List all citations
- `GET /api/citations/{id}` - Get citation by ID
- `POST /api/citations` - Create new citation
- `PUT /api/citations/{id}` - Update citation
- `DELETE /api/citations/{id}` - Delete citation

### Users
- `GET /api/users` - List all users (admin only)
- `GET /api/users/{id}` - Get user by ID
- `POST /api/users` - Create new user
- `PUT /api/users/{id}` - Update user
- `DELETE /api/users/{id}` - Delete user

### Authentication
- All endpoints require Privy authentication tokens in the `Authorization` header

## Development

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test module
cargo test --test publications
```

### Code Formatting

```bash
cargo fmt
```

### Linting

```bash
cargo clippy
```

### Database Operations

```bash
# Create new migration
sqlx migrate add migration_name

# Run migrations
sqlx migrate run

# Revert last migration
sqlx migrate revert
```

## Docker Services Management

### Start Services
```bash
docker-compose up -d
```

### Stop Services
```bash
docker-compose down
```

### View Logs
```bash
docker-compose logs -f
```

### Reset Everything
```bash
docker-compose down -v
docker-compose up -d
```

## Accessing Services

- **PostgreSQL**: `localhost:6500` (username: `postgres`, password: `postgres`)
- **pgAdmin**: `http://localhost:5050` (email: `admin@admin.com`, password: `admin`)
- **Redis**: `localhost:6379`
- **MinIO Console**: `http://localhost:9001` (username: `minioadmin`, password: `minioadmin`)
- **API Server**: `http://localhost:8080`

## Environment Variables Reference

| Variable | Description | Default |
|----------|-------------|---------|
| `DATABASE_URL` | PostgreSQL connection string | `postgres://postgres:postgres@localhost:6500/publish3` |
| `REDIS_URL` | Redis connection URL | `redis://localhost:6379` |
| `SERVER_ADDRESS` | Server bind address | `0.0.0.0` |
| `SERVER_PORT` | Server port | `8080` |
| `SERVER_BASE_URL` | Base URL for the server | `http://localhost:8080` |
| `CLIENT_ORIGIN` | Allowed CORS origin | `http://localhost:3000` |
| `S3_ACCESS_KEY` | S3/MinIO access key | `minioadmin` |
| `S3_SECRET_KEY` | S3/MinIO secret key | `minioadmin` |
| `S3_ENDPOINT` | S3/MinIO endpoint | `http://localhost:9000` |
| `PRIVY_APP_ID` | Privy application ID | - |
| `PRIVY_APP_SECRET` | Privy application secret | - |
| `PRIVY_JWT_VERIFICATION_KEY` | Base64-encoded JWT verification key | - |

## Troubleshooting

### Common Issues

1. **Database connection errors**: Ensure PostgreSQL is running via Docker Compose
2. **Migration errors**: Run `sqlx migrate run` to apply migrations
3. **Redis connection errors**: Check if Redis container is running
4. **S3/MinIO errors**: Verify MinIO is accessible at `http://localhost:9000`
5. **CORS errors**: Update `CLIENT_ORIGIN` in `.env` to match your frontend URL

### Logs

Check application logs:
```bash
# Set log level
RUST_LOG=debug cargo run

# Or check Docker logs
docker-compose logs -f postgres
docker-compose logs -f redis
docker-compose logs -f minio
```

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License.

## Acknowledgments

- [Actix-web](https://actix.rs/) - Rust web framework
- [SQLx](https://github.com/launchbadge/sqlx) - Async PostgreSQL driver
- [Privy](https://www.privy.io/) - Authentication service
- [MinIO](https://min.io/) - S3-compatible object storage
