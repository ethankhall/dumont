FROM rust:1.57.0-buster as rust-builder

WORKDIR /dumont

# copy over your manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

RUN USER=root cargo new --bin dumont-web-server
COPY dumont-web-server/Cargo.toml /dumont/dumont-web-server/Cargo.toml
WORKDIR /dumont

RUN cargo build --release
RUN rm dumont-web-server/src/*.rs
RUN rm target/release/deps/dumont*

ADD dumont-web-server /dumont/dumont-web-server

# this build step will cache your dependencies
RUN cargo build --release
RUN mkdir /app && mv target/release/dumont-web-server /app/dumont
RUN /app/dumont --help

# verify linked deps
FROM debian:buster-slim

RUN apt-get update && apt-get install -y libpq5 && apt-get clean 

# copy the build artifact from the build stage
COPY --from=rust-builder /app/dumont /app/dumont
RUN /app/dumont --help
RUN ls -alh /app/dumont

# our final base
FROM debian:buster-slim

RUN apt-get update && apt-get install -y tini libpq5 && apt-get clean 

# copy the build artifact from the build stage
COPY --from=rust-builder /app/dumont /app/dumont

WORKDIR /app
ENV PATH $PATH:/app

ENTRYPOINT ["/usr/bin/tini", "--"]
# set the startup command to run your binary
CMD [ "/app/dumont"]
