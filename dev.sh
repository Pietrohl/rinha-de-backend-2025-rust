#!/usr/bin/env bash
set -euo pipefail

# Load env variables from .env.local
if [[ -f ".env.local" ]]; then
  set -o allexport
  source .env.local
  set +o allexport
fi

# Clean up on exit
cleanup() {
  echo -e "\nStopping docker-compose..."
  docker-compose -f docker-compose-dev.yml down
}
trap cleanup EXIT

# Start docker-compose (foreground logs)
docker-compose -f docker-compose-dev.yml up &
DOCKER_PID=$!

# Start cargo watch
cargo watch -x run &
CARGO_PID=$!

# Forward Ctrl+C to children
trap "kill $DOCKER_PID $CARGO_PID" SIGINT

# Wait for both
wait -n
