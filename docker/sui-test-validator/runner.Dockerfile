# Production Image
FROM debian:bullseye AS runtime
# Use jemalloc as memory allocator
RUN apt-get update && apt-get install -y libjemalloc-dev libpq-dev htop curl nginx
ENV LD_PRELOAD /usr/lib/x86_64-linux-gnu/libjemalloc.so
ARG PROFILE=release
WORKDIR sui
# # Both bench and release profiles copy from release dir
# COPY --from=builder /sui/target/release/sui-test-validator /usr/local/bin
# COPY --from=builder /sui/target/release/sui /usr/local/bin
# # Staged migration from /usr/local/bin to /opt/sui/bin
# COPY --from=builder /sui/target/release/sui-test-validator /opt/sui/bin
# COPY --from=builder /sui/target/release/sui /opt/sui/bin
EXPOSE 9000
EXPOSE 9184
EXPOSE 9123
EXPOSE 9124
ARG BUILD_DATE
ARG GIT_REVISION
LABEL build-date=$BUILD_DATE
LABEL git-revision=$GIT_REVISION
