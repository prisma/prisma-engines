FROM rust:1.70.0
ARG db_url
WORKDIR /usr/src/doctor
COPY doctor/src ./src
COPY doctor/Cargo.toml ./Cargo.toml
RUN cargo install --path .

ENV DATABASE_URL=$db_url
ENV RUST_LOG=debug
ENV PORT 8080
EXPOSE 8080
CMD ["doctor"]