FROM rust:1.75 as builder

WORKDIR /usr/src/friends-connect
COPY . .

RUN cargo build --release

FROM debian:bookworm-slim

COPY --from=builder /usr/src/friends-connect/target/release/friends-connect /usr/local/bin/friends-connect
COPY --from=builder /usr/src/friends-connect/static /static

EXPOSE 8080

CMD ["friends-connect"]