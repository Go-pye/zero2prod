#!/usr/bin/env bash
set -x
set -eo pipefail

if ! [ -x "$(command -v sqlx)" ]; then
  echo >&2 "Error: sqlx is not installed."
  echo >&2 "Use:"
  echo >&2 "    cargo install --version='~0.8' sqlx-cli --no-default-features --features rustls,postgres"
  echo >&2 "to install it."
  exit 1
fi

# Check if port 5432 is in use and stop local PostgreSQL if needed
if lsof -i :5432 >/dev/null 2>&1; then
    echo "Port 5432 is already in use. Stopping local PostgreSQL..."
    brew services stop postgresql@14 || true
    sleep 2
fi

# Check if a custom parameter has been set, otherwise use default values
DB_PORT="${DB_PORT:=5432}"
DB_USER="${POSTGRES_USER:=postgres}"
DB_PASSWORD="${POSTGRES_PASSWORD:=password}"
DB_NAME="${POSTGRES_DB:=newsletter}"
DB_HOST="${POSTGRES_HOST:=localhost}"

# Clean up existing containers
echo "Cleaning up existing containers..."
docker ps -a | grep postgres | awk '{print $1}' | xargs -r docker rm -f || true

# Launch postgres using Docker
CONTAINER_NAME="newsletter-db"
echo "Starting PostgreSQL container..."
docker run \
    --name "${CONTAINER_NAME}" \
    -e POSTGRES_USER=${DB_USER} \
    -e POSTGRES_PASSWORD=${DB_PASSWORD} \
    -e POSTGRES_DB=${DB_NAME} \
    -p "${DB_PORT}":5432 \
    --health-cmd="pg_isready -U ${DB_USER}" \
    --health-interval=1s \
    --health-timeout=5s \
    --health-retries=10 \
    -d postgres:14

# Wait for container to be healthy
echo "Waiting for PostgreSQL to become healthy..."
until [ "$(docker inspect -f {{.State.Health.Status}} ${CONTAINER_NAME} 2>/dev/null)" == "healthy" ]; do
    echo "Waiting for PostgreSQL to become healthy..."
    sleep 1;
done

echo "PostgreSQL is healthy!"

# Update pg_hba.conf to allow password authentication from host
echo "Configuring PostgreSQL..."
docker exec -i "${CONTAINER_NAME}" bash -c 'echo "host all all all md5" >> /var/lib/postgresql/data/pg_hba.conf'
docker exec -i "${CONTAINER_NAME}" bash -c 'echo "listen_addresses = '\''*'\''" >> /var/lib/postgresql/data/postgresql.conf'

# Restart to apply configuration changes
echo "Restarting PostgreSQL to apply configuration..."
docker restart "${CONTAINER_NAME}"

# Wait for container to be healthy again
echo "Waiting for PostgreSQL to become healthy after restart..."
until [ "$(docker inspect -f {{.State.Health.Status}} ${CONTAINER_NAME} 2>/dev/null)" == "healthy" ]; do
    echo "Waiting for PostgreSQL to become healthy..."
    sleep 1;
done

# Wait a moment for the server to be ready
sleep 2

# Set DATABASE_URL for migrations
export DATABASE_URL="postgres://${DB_USER}:${DB_PASSWORD}@${DB_HOST}:${DB_PORT}/${DB_NAME}"

# Verify connection
echo "Verifying connection..."
for i in {1..5}; do
    if PGPASSWORD="${DB_PASSWORD}" psql -h "${DB_HOST}" -p "${DB_PORT}" -U "${DB_USER}" -d "${DB_NAME}" -c "\conninfo"; then
        break
    fi
    echo "Connection attempt $i failed, retrying..."
    sleep 2
done

# Run migrations
echo "Running migrations..."
sqlx migrate run

echo "Migrations complete!"