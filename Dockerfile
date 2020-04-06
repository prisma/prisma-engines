FROM rust:1.42.0
MAINTAINER Julius de Bruijn <bruijn@prisma.io>

ENV USER root

RUN apt-get -y update
RUN apt-get -y install libssl-dev build-essential

ENV SERVER_ROOT=/usr/src/query-engine
ENV RUST_LOG_FORMAT=devel
ENV RUST_BACKTRACE=1
ENV RUST_LOG=query_engine=debug,quaint=debug,query_core=debug,query_connector=debug,sql_query_connector=debug,prisma_models=debug,engineer=debug
ENV PATH="/root/.cargo/bin:${PATH}"

ADD . /usr/src/query-engine
WORKDIR /usr/src/query-engine/

RUN cargo build --release
RUN mv target/release/query-engine /usr/bin
RUN mv target/release/migration-engine /usr/bin

WORKDIR /

RUN rm -rf /usr/src
CMD /usr/bin/query-engine --host 0.0.0.0
