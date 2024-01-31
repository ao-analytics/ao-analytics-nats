FROM rust as builder

ENV SQLX_OFFLINE=true

WORKDIR /aodata-nats-orders
COPY . .
COPY .env.prod .env
RUN cargo install --path .

FROM ubuntu:latest

COPY --from=builder /usr/local/cargo/bin/aodata-nats-orders /usr/local/bin/aodata-nats-orders

CMD ["aodata-nats-orders"]