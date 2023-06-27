#!/bin/bash
cargo run &
sleep 1
open http://localhost:8080/slow-queries &
