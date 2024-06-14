FROM lukemathwalker/cargo-chef:latest as chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare

FROM chef AS builder
COPY --from=planner /app/recipe.json .
RUN cargo chef cook --release
COPY . .
RUN cargo build --release

FROM debian:stable-slim AS runtime
# For example sqlite:/persistent/production.sqlite?mode=rwc
ARG DATABASE_URL
WORKDIR /app
COPY --from=builder /app/target/release/olmonoko-backend /app
COPY --from=builder /app/static /app/static
COPY --from=builder /app/templates /app/templates
CMD ["/app/olmonoko-backend"]
