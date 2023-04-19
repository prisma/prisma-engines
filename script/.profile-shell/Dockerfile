ARG RUST_VERSION
FROM rust:$RUST_VERSION-buster
RUN printf "deb http://ftp.de.debian.org/debian buster main" >> /etc/apt/sources.list 
RUN apt update
RUN apt-get install -y git valgrind linux-perf strace

RUN curl -fsSL https://fnm.vercel.app/install | bash
RUN source ~/.bashrc
RUN fnm install v16

# Symlink caches
RUN mkdir -p /.cargo/registry
RUN mkdir .cargo/git
RUN rm -rf /usr/local/cargo/git \
    /usr/local/cargo/registry \
    /usr/local/cargo/.package-cache \
    /usr/local/cargo/.crates.toml \
    /usr/local/cargo/.crates2.json
RUN ln -s /.cargo/git /usr/local/cargo
RUN ln -s /.cargo/registry /usr/local/cargo
RUN ln -s /.cargo/.package-cache /usr/local/cargo
RUN ln -s /.cargo/.crates.toml /usr/local/cargo
RUN ln -s /.cargo/.crates2.json /usr/local/cargo

# Set env
ENV PATH="${PATH}:/prisma-engines/script/.profile-shell/bin"
ENV VALGRIND_LIB=/usr/lib/aarch64-linux-gnu/valgrind/
ENV CARGO_TARGET_DIR=target-alternatives

# Install cargo-flamegraph
RUN cargo install flamegraph

CMD ["/bin/bash"]