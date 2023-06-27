#!/bin/bash
docker compose -f ../docker-compose.yml up -d postgres-doctor
cargo run &
sleep 1
open http://localhost:8080/slow-queries
