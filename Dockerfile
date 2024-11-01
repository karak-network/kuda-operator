FROM rust:slim AS builder

RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev protobuf-compiler

WORKDIR /operator
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && \
    apt-get install -y libssl-dev ca-certificates && \
    rm -rf /var/lib/apt/lists/*
COPY --from=builder /operator/target/release/kuda-operator /kuda-operator
EXPOSE 8080
CMD ["/kuda-operator", "run"]
