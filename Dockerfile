FROM lukemathwalker/cargo-chef:latest as planner

WORKDIR /ao-analytics-nats
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM lukemathwalker/cargo-chef:latest as builder

ENV SQLX_OFFLINE=true
WORKDIR /ao-analytics-nats
COPY --from=planner /ao-analytics-nats/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release

FROM ubuntu:latest

COPY --from=builder /ao-analytics-nats/target/release/ao-analytics-nats /usr/local/bin/ao-analytics-nats

CMD ["ao-analytics-nats"]