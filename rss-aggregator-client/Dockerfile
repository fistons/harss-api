FROM rust:1.56-slim AS builder

ARG DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y libssl-dev pkg-config
RUN rustup target add wasm32-unknown-unknown
RUN cargo install trunk wasm-bindgen-cli
RUN ls -alsh

WORKDIR app
ADD . .

RUN trunk build --release

FROM nginx:alpine

COPY --from=builder /app/dist/* /usr/share/nginx/html/
