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
COPY .env.prod .env
RUN cargo build --release

FROM ubuntu:latest

EXPOSE 8080
COPY --from=builder /ao-analytics-nats/target/release/ao-analytics-nats /usr/local/bin/ao-analytics-nats

CMD ["ao-analytics-nats"]