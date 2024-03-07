#!/bin/bash

# Start limitador in the background
/bench/limitador-server /bench/limits.yaml memory &

# Wait for limitador to start
sleep 5

# Run limitador-driver
/bench/limitador-driver rpc://127.0.0.1:8081 3m
