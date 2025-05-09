FROM lukemathwalker/cargo-chef:latest AS chef
WORKDIR /app
RUN apt-get update && apt install clang -y

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
ENV SQLX_OFFLINE=true
RUN cargo build --release

FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends ca-certificates postgresql-client \
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/zero2prod zero2prod
COPY configuration configuration
ENV APP_ENVIRONMENT=production
# Add explicit environment variables for database connection
# These will be overridden by Digital Ocean's environment variables if set
ENV APP_DATABASE__HOST=127.0.0.1
ENV APP_DATABASE__PORT=5432
ENV APP_DATABASE__USERNAME=postgres
ENV APP_DATABASE__DATABASE_NAME=newsletter
# Add debugging to see environment variables at startup
ENTRYPOINT ["sh", "-c", "echo 'Testing database connection...' && PGPASSWORD=$APP_DATABASE__PASSWORD psql -h $APP_DATABASE__HOST -p $APP_DATABASE__PORT -U $APP_DATABASE__USERNAME -d $APP_DATABASE__DATABASE_NAME -c 'SELECT 1' || echo 'Database connection test failed' && env | grep -i app_ && ./zero2prod"]