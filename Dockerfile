FROM rust:1.68-alpine3.17 as builder

RUN apk add --no-cache git
RUN USER=root cargo new --bin tz-proto-vanity
WORKDIR /tz-proto-vanity
COPY ./Cargo.toml ./Cargo.toml
RUN cargo build --release
RUN rm src/*.rs

ADD . ./

RUN rm ./target/release/deps/tz_proto_vanity*
RUN cargo build --release


FROM alpine:3.17
ARG APP_DIR=/opt/
RUN mkdir -p ${APP_DIR}
COPY --from=builder /tz-proto-vanity/target/release/tz-proto-vanity ${APP_DIR}/
WORKDIR ${APP_DIR}

ENTRYPOINT ["./tz-proto-vanity"]
