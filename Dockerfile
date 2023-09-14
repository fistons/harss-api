FROM debian:12-slim
ARG TARGETARCH
LABEL maintainer=eric@pedr0.net
ENV DEBIAN_FRONTEND=noninteractive

RUN adduser rss-aggregator

RUN apt-get update && apt-get install -y curl tzdata libc6 && rm -rf /var/lib/apt/lists/*
RUN cp /usr/share/zoneinfo/Europe/Paris /etc/localtime

COPY $TARGETARCH/rss-aggregator /usr/local/bin/rss-aggregator
COPY static/ static/
RUN chmod +x /usr/local/bin/rss-aggregator

EXPOSE 8080
USER rss-aggregator
ENTRYPOINT ["rss-aggregator"]
