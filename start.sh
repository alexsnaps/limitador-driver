#!/bin/bash

# Default values
DEFAULT_STORAGE="memory"
DEFAULT_STORAGE_URL=""
DEFAULT_TIME="3m"

storage=$DEFAULT_STORAGE
storage_url=$DEFAULT_STORAGE_URL
time=$DEFAULT_TIME

# Function to parse key-value pairs
parse_arguments() {
    echo "Parsing arguments"
    for argument in "$@"
    do
        key=$(echo $argument | cut -f1 -d=)
        value=$(echo $argument | cut -f2- -d=)

        case "$key" in
            "storage")        storage="$value" ;;
            "storage_url")    storage_url="$value" ;;
            "time")           time="$value" ;;
            *)             echo "Unknown argument: $key" ;;
        esac
    done
}

parse_arguments "$@"

echo "Executing limitador: $ limitador-server /bench/limits.yaml $storage $storage_url"

# Start limitador in the background
if [[ -z "$storage_url" ]]; then
    /bench/limitador-server /bench/limits.yaml "$storage" &
else
    /bench/limitador-server /bench/limits.yaml "$storage" "$storage_url" &
fi

# Wait for limitador to start
sleep 5

echo "Executing limitador-driver: $ limitador-driver rpc://127.0.0.1:8081 $time"

# Run limitador-driver
/bench/limitador-driver rpc://127.0.0.1:8081 "$time"
