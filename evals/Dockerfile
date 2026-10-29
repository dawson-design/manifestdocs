# Images for eval sessions. Build from the repo root; Dockerfile.dockerignore limits the
# context to the crates and the plugin, so nothing from evals/ reaches an image.
#
#   docker build -f evals/Dockerfile --target arm-a -t mfd-eval:a .   # agent CLI only
#   docker build -f evals/Dockerfile --target arm-b -t mfd-eval:b .   # plus manifestdocs and the plugin
#
# run.sh builds both and runs each session in a throwaway container from one of them.

ARG RUST_IMAGE=rust:1.94.0-bookworm@sha256:365468470075493dc4583f47387001854321c5a8583ea9604b297e67f01c5a4f
ARG NODE_IMAGE=node:24.21.0-bookworm-slim@sha256:d6aa754f16b3197301076f047b5def2f02ea1dbbc2ca920407d46d7ec7f87b20

FROM ${RUST_IMAGE} AS mfd-build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo build --release --locked -p manifestdocs

FROM ${NODE_IMAGE} AS arm-a
ARG CLAUDE_CODE_VERSION=2.1.312
RUN apt-get update \
  && apt-get install -y --no-install-recommends \
    python3=3.11.2-1+b1 git=1:2.39.5-0+deb12u3 ca-certificates=20250419~deb12u2 \
  && rm -rf /var/lib/apt/lists/* \
  && npm install -g "@anthropic-ai/claude-code@${CLAUDE_CODE_VERSION}" \
  && npm cache clean --force \
  # The work folder is a bind mount owned by the host user.
  && git config --system safe.directory '*'
ENV DISABLE_AUTOUPDATER=1 \
    CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1 \
    PYTHONDONTWRITEBYTECODE=1
USER node
WORKDIR /work
ENTRYPOINT ["claude"]

FROM arm-a AS arm-b
USER root
COPY --from=mfd-build /src/target/release/manifestdocs /src/target/release/mfd /usr/local/bin/
# A user installs the plugin from the local marketplace in a clone of the repo; this is the
# part of the clone that the marketplace reads.
COPY .claude-plugin /opt/manifestdocs/.claude-plugin
COPY plugins/manifestdocs /opt/manifestdocs/plugins/manifestdocs
USER node
RUN claude plugin marketplace add /opt/manifestdocs \
  && claude plugin install manifestdocs@manifestdocs
