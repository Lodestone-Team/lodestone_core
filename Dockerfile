### Pre-Configure the image for re-use.
FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

### Stage 1.
# Compute the receipe for the dependencies.
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

### Stage 2.
# Build dem binaries.
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json 

COPY . .
RUN cargo build --release --bin main --features "vendored-openssl" --locked

### Stage 3.
# Doing the last things to setup the lodestone server.
FROM debian:bullseye-slim AS runtime
SHELL ["/bin/bash", "-o", "pipefail", "-c"]

ARG UID=2000
ARG USER=lodestone

RUN apt-get update \
  && apt-get install -y ca-certificates \
  && update-ca-certificates \
  && rm -rf /var/lib/apt/lists/*

RUN useradd -ms /bin/bash $USER && usermod -u $UID $USER
USER $USER

RUN mkdir -p /home/$USER/.lodestone
COPY --from=builder /app/target/release/main /usr/local/bin

EXPOSE 16662
CMD ["/usr/local/bin/main"]
