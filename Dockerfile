FROM rust as builder

ENV SQLX_OFFLINE=true

WORKDIR /ao-analytics-nats
COPY . .
COPY .env.prod .env
RUN cargo install --path .

FROM ubuntu:latest

COPY --from=builder /usr/local/cargo/bin/ao-analytics-nats /usr/local/bin/ao-analytics-nats

CMD ["ao-analytics-nats"]