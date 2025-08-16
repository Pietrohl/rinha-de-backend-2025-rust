FROM rust:1.88.0 as base

WORKDIR /app

FROM base AS builder
RUN mkdir /temp
COPY Cargo.toml /temp
COPY Cargo.lock /temp
COPY src /temp/src

RUN cd /temp && cargo build --release


FROM base as release

COPY --from=builder temp/target/release/rinha-rust rinha-rust

EXPOSE 3000

CMD ["./rinha-rust"]
