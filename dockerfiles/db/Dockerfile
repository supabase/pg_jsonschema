FROM postgres:16
RUN apt-get update

ENV build_deps ca-certificates \
  git \
  build-essential \
  libpq-dev \
  postgresql-server-dev-16 \
  curl \
  libreadline6-dev \
  zlib1g-dev


RUN apt-get install -y --no-install-recommends $build_deps pkg-config cmake

WORKDIR /home/supa

ENV HOME=/home/supa \
  PATH=/home/supa/.cargo/bin:$PATH
RUN chown postgres:postgres /home/supa
USER postgres

RUN \
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path --profile minimal --default-toolchain stable && \
  rustup --version && \
  rustc --version && \
  cargo --version

# PGX
RUN cargo install cargo-pgrx --version 0.12.5 --locked

RUN cargo pgrx init --pg16 $(which pg_config)

USER root

COPY . .
RUN cargo pgrx install

RUN chown -R postgres:postgres /home/supa
RUN chown -R postgres:postgres /usr/share/postgresql/16/extension
RUN chown -R postgres:postgres /usr/lib/postgresql/16/lib

USER postgres
