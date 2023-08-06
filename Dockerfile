FROM debian:12-slim AS api
ARG TARGETARCH
LABEL maintainer=eric@pedr0.net
ENV DEBIAN_FRONTEND=noninteractive

RUN adduser rss-fetcher

RUN apt-get update && apt-get install -y curl tzdata libc6 && rm -rf /var/lib/apt/lists/*
RUN cp /usr/share/zoneinfo/Europe/Paris /etc/localtime

COPY $TARGETARCH/rss-aggregator /usr/local/bin/rss-aggregator
COPY crates/api/static/ static/

EXPOSE 8080
USER rss-fetcher
ENTRYPOINT ["rss-aggregator"]

FROM debian:12-slim as fetcher
ARG TARGETARCH
LABEL maintainer=eric@pedr0.net
ENV DEBIAN_FRONTEND=noninteractive

RUN adduser rss-fetcher

RUN apt-get update && apt-get install -y curl tzdata libc6 && rm -rf /var/lib/apt/lists/*
RUN cp /usr/share/zoneinfo/Europe/Paris /etc/localtime

COPY $TARGETARCH/fetcher /usr/local/bin/fetcher

EXPOSE 8080
USER rss-fetcher
ENTRYPOINT ["fetcher"]
