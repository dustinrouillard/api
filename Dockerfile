FROM clux/muslrust AS builder

RUN apt update
RUN apt install wget -y
RUN wget http://archive.ubuntu.com/ubuntu/pool/main/o/openssl/libssl1.1_1.1.0g-2ubuntu4_amd64.deb
RUN dpkg -i libssl1.1_1.1.0g-2ubuntu4_amd64.deb

WORKDIR /app

COPY . .

RUN cargo prisma generate
RUN cargo build --release

FROM alpine

LABEL org.opencontainers.image.source=https://github.com/dustinrouillard/api

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/api /app

CMD ["/app"]