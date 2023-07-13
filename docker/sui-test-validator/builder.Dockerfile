FROM rust:1.70.0 AS chef
ARG PROFILE=release
WORKDIR sui
RUN apt-get update && apt-get install -y cmake clang libpq-dev git libssl-dev
RUN apt-get install -y protobuf-compiler libprotobuf-dev

# For now disable caching

# Plan out the 3rd-party dependencies that need to be built.
#
# This is done by:
#   1. Copy in Cargo.toml, Cargo.lock, and the workspace-hack crate
#   2. Removing all workspace crates, other than the workpsace-hack
#      crate, from the workspace Cargo.toml file.
#   3. Update the lockfile in order to reflect the changes to the
#      root Cargo.toml file.
# FROM chef AS planner
# COPY Cargo.toml Cargo.lock ./
# COPY crates/workspace-hack crates/workspace-hack
# RUN sed -i '/crates\/workspace-hack/b; /crates/d; /narwhal/d' Cargo.toml \
#     && cargo metadata -q >/dev/null

# Build and cache all dependencies.
#
# In a fresh layer, copy in the "plan" generated by the planner
# and run `cargo build` in order to create a caching Docker layer
# with all dependencies built.
FROM chef AS builder 
ARG PROFILE=release
# COPY --from=planner /sui/Cargo.toml Cargo.toml
# COPY --from=planner /sui/Cargo.lock Cargo.lock
# COPY --from=planner /sui/crates/workspace-hack crates/workspace-hack
# RUN cargo build --profile $PROFILE

# Build application
#
# Copy in the rest of the crates (and an unmodified Cargo.toml and Cargo.lock)
# and build the application. At this point no dependencies should need to be
# built as they were built and cached by the previous layer.
# ARG GIT_REVISION
# COPY Cargo.toml Cargo.lock ./
# COPY crates crates
# COPY sui-execution sui-execution
# COPY narwhal narwhal
# COPY external-crates external-crates
# RUN cargo build --profile $PROFILE --bin sui-test-validator --bin sui
# RUN apt-get update && apt-get install -y binutils
# RUN strip --debug-only -w -K sui* -K narwhal* -K hyper* -K typed_store* "/sui/target/$PROFILE/sui-node"

