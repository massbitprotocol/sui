#!/bin/bash
DIR="$( cd "$( dirname "$0" )" && pwd )"
git config --global --add safe.directory /sui
REPO_ROOT="$(git rev-parse --show-toplevel)"
GIT_REVISION="$(git describe --always --dirty --exclude '*')"
BUILD_DATE="$(date -u +'%Y-%m-%d')"
#PROFILE=release
PROFILE=dev
echo
echo "Building sui-test-validator"
echo "docker context: $REPO_ROOT"
echo "build date: \t$BUILD_DATE"
echo "git revision: \t$GIT_REVISION"
echo

cargo build --manifest-path ${DIR}/Cargo.toml --profile $PROFILE --bin sui-test-validator --bin sui #--target aarch64-apple-darwin
cargo build --manifest-path ${DIR}/scalar/tofnd/Cargo.toml --profile $PROFILE
# COPY /sui/target/release/sui-test-validator /usr/local/bin
# COPY /sui/target/release/sui /usr/local/bin