FROM rust:1.57.0-buster as rust-builder

WORKDIR /dumont

# copy over your manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

RUN USER=root cargo new --bin dumont-webserver
COPY dumont-webserver/Cargo.toml /dumont/dumont-webserver/Cargo.toml
WORKDIR /dumont

RUN cargo build --release
RUN rm dumont-webserver/src/*.rs
RUN rm target/release/deps/dumont*

ADD dumont-webserver /dumont/dumont-webserver

# this build step will cache your dependencies
RUN cargo build --release
RUN mkdir /app && mv target/release/dumont-bin /app/dumont-webserver
RUN /app/dumont-webserver --help

# verify linked deps
FROM debian:buster-slim

RUN apt-get update && apt-get install -y libpq5 && apt-get clean 

# copy the build artifact from the build stage
COPY --from=rust-builder /app/dumont-webserver /app/dumont-webserver
RUN /app/dumont-webserver --help

# our final base
FROM debian:buster-slim

RUN apt-get update && apt-get install -y libpq5 && apt-get clean 

# copy the build artifact from the build stage
COPY --from=rust-builder /app/dumont-webserver /app/dumont-webserver

ENV TINI_VERSION v0.18.0
ADD https://github.com/krallin/tini/releases/download/${TINI_VERSION}/tini /tini
RUN chmod +x /tini

WORKDIR /app

ENTRYPOINT ["/tini", "--"]
# set the startup command to run your binary
CMD [ "/app/dumont-webserver"]
