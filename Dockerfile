FROM clux/muslrust AS builder

WORKDIR /app

COPY . .

RUN cargo prisma generate
RUN cargo build --release

FROM alpine

LABEL org.opencontainers.image.source=https://github.com/dustinrouillard/api

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/api /app

CMD ["/app"]
