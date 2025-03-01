# Build Stage
FROM --platform=linux/amd64 rustlang/rust:nightly as builder

ENV DEBIAN_FRONTEND=noninteractive
## Install build dependencies.
RUN apt-get update 
RUN apt-get install -y cmake clang
RUN cargo install cargo-fuzz

## Add source code to the build stage.
ADD . /parcel-css/

# needed to fix bug with nightly version not parsing cargo.toml
WORKDIR /parcel-css/
RUN sed '/serde = \[\"smallvec\/serde\", \"cssparser\/serde\"\]/d' Cargo.toml > Cargo.toml2
RUN mv Cargo.toml2 Cargo.toml

WORKDIR /parcel-css/fuzz/

RUN cargo +nightly fuzz build

FROM --platform=linux/amd64 rustlang/rust:nightly

## TODO: Change <Path in Builder Stage>
COPY --from=builder /parcel-css/fuzz/target/x86_64-unknown-linux-gnu/release/filename /
COPY --from=builder /parcel-css/fuzz/target/x86_64-unknown-linux-gnu/release/parser /
