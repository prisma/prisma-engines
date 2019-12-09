FROM rust:latest
MAINTAINER Julius de Bruijn <bruijn@prisma.io>

ENV USER root

RUN apt-get -y update
RUN apt-get -y install libssl-dev build-essential

ENV SERVER_ROOT=/usr/src/prisma-engine
ENV RUST_LOG_FORMAT=devel
ENV RUST_BACKTRACE=1
ENV RUST_LOG=prisma=info,quaint=info,query_core=info,query_connector=info,sql_query_connector=info,prisma_models=info,engineer=info
ENV PATH="/root/.cargo/bin:${PATH}"

ADD . /usr/src/prisma-engine
WORKDIR /usr/src/prisma-engine/

RUN cargo build --release
RUN mv target/release/prisma /usr/bin
RUN mv target/release/migration-engine /usr/bin

WORKDIR /

RUN rm -rf /usr/src
CMD /usr/bin/prisma
