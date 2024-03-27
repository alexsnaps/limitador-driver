# Stage 1: Build limitador-driver
FROM rust:1.76-bullseye as builder

RUN apt update && apt upgrade -y \
    && apt install -y protobuf-compiler

WORKDIR /usr/src/limitador-driver
COPY . .
RUN cargo build --release

# Stage 2: Setup the environment to run limitador-driver and limitador
FROM rust:1.76-bullseye

WORKDIR /bench

# Copy the Rust binary from the builder stage
COPY --from=builder /usr/src/limitador-driver/target/release/limitador-driver /bench/

# Copy the limits.yaml to configure limitador
COPY limits.yaml /bench

# Copy the limitador service into the container
COPY --from=quay.io/kuadrant/limitador:write-behind-lock /home/limitador/bin/limitador-server /bench/limitador-server

# Copy the script to start both services
COPY start.sh /bench/start.sh
RUN chmod +x /bench/start.sh

# Run the script
ENTRYPOINT ["/bin/bash", "-c", "/bench/start.sh \"$@\"", "--"]
