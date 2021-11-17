FROM rust:1.56.0

COPY ./ ./

RUN cargo build --release

ENV host "127.0.0.1"
ENV port 8755

CMD ["sh", "-c", "./target/release/three_em start --host ${host} --port ${port}"]
