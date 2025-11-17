# Build stage
FROM rust:latest as builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies (this layer will be cached)
RUN cargo build --release && rm -rf src

# Copy source code
COPY src ./src
COPY migrations ./migrations

# Build the application
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/target/release/slackmud /app/slackmud
COPY --from=builder /app/migrations /app/migrations

# Create directory for database
RUN mkdir -p /app/data

# Set environment variables
ENV DATABASE_URL=sqlite:///app/data/slackmud.db
ENV HOST=0.0.0.0
ENV PORT=3000

EXPOSE 3000

CMD ["/app/slackmud"]
