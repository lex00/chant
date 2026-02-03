# Build stage
FROM rust:1.83-slim as builder

WORKDIR /app

# Install musl-tools for static linking
RUN apt-get update && \
    apt-get install -y musl-tools && \
    rm -rf /var/lib/apt/lists/*

# Add musl target
RUN rustup target add x86_64-unknown-linux-musl

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build the application with musl for static linking
RUN cargo build --release --target x86_64-unknown-linux-musl

# Runtime stage
FROM alpine:3.19

# Install CA certificates for HTTPS requests
RUN apk add --no-cache ca-certificates

# Copy the binary from builder
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/chant /usr/local/bin/chant

# Create a non-root user
RUN adduser -D -u 1000 chant

USER chant
WORKDIR /home/chant

ENTRYPOINT ["/usr/local/bin/chant"]
CMD ["--help"]
