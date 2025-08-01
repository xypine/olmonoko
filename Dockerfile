FROM clux/muslrust:nightly AS planner
WORKDIR /app
# We only pay the installation cost once,
# it will be cached from the second build onwards
RUN cargo install cargo-chef
COPY . .
RUN rustup show
RUN cargo chef prepare --recipe-path recipe.json

# Build dependencies - this is the caching Docker layer!
FROM clux/muslrust:nightly AS cacher
WORKDIR /app
RUN cargo install cargo-chef
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json

# Build the application
FROM clux/muslrust:nightly AS builder
WORKDIR /app
COPY . .
COPY --from=cacher /app/target target
RUN cargo build --release --target x86_64-unknown-linux-musl

FROM gcr.io/distroless/static AS runtime
# For example sqlite://persistent/production.sqlite?mode=rwc
ARG DATABASE_URL
WORKDIR /app
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/olmonoko-backend /app
COPY --from=builder /app/olmonoko-backend/static /app/static
COPY --from=builder /app/olmonoko-backend/templates /app/templates
CMD ["/app/olmonoko-backend"]
