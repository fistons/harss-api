FROM debian:12-slim
ARG TARGETARCH
LABEL maintainer=eric@pedr0.net
ENV DEBIAN_FRONTEND=noninteractive

RUN adduser harss-api

RUN apt-get update && apt-get install -y curl tzdata libc6 && rm -rf /var/lib/apt/lists/*
RUN cp /usr/share/zoneinfo/Europe/Paris /etc/localtime

COPY $TARGETARCH/harss-api /usr/local/bin/harss-api
COPY static/ static/
RUN chmod +x /usr/local/bin/harss-api

EXPOSE 8080
USER harss-api
ENTRYPOINT ["harss-api"]
