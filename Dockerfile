# Two stages: build the binary, then ship it on its own.
#
# `web/dist` is committed, so the build needs Rust and nothing else — no Node,
# no network beyond crates.io.

FROM rust:1.97-bookworm AS build

WORKDIR /src
COPY . .
RUN cargo build --release --locked

FROM debian:bookworm-slim

# SQLite is compiled into the binary; what is left is TLS roots for outbound
# calls and a place to put the database.
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/*

COPY --from=build /src/target/release/fugantt /usr/local/bin/fugantt
COPY LICENSE NOTICE /usr/share/doc/fugantt/

# Listen on every interface: inside a container there is nothing else to reach.
ENV HOST=0.0.0.0 PORT=3000 FUGANTT_DB=/data/fugantt.db
VOLUME /data
EXPOSE 3000

# Not root. The volume is the only thing it needs to write.
RUN useradd --system --uid 10001 fugantt && mkdir -p /data && chown fugantt /data
USER fugantt

CMD ["fugantt"]
