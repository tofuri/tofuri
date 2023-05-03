FROM rust:latest as build
WORKDIR /usr/src/tofuri
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    clang \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*
COPY . .
RUN cargo build --bin tofuri --release
FROM debian:stable-slim
COPY --from=build /usr/src/tofuri/target/release/tofuri /usr/local/bin/
EXPOSE 2020 2021
ENTRYPOINT ["tofuri"]