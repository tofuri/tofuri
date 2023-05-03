FROM rust:latest as builder
WORKDIR /a
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    clang \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*
COPY . .
RUN cargo build --bin tofuri --release
FROM debian:stable-slim
COPY --from=builder /a/target/release/tofuri /usr/local/bin/
EXPOSE 2020 2021
ENTRYPOINT ["tofuri"]