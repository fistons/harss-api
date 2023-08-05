[![Check, test, everything](https://github.com/fistons/rss-aggregator/actions/workflows/test.yml/badge.svg?branch=devel)](https://github.com/fistons/rss-aggregator/actions/workflows/test.yml)

# RSS Aggregator

RSS, but in rust

## Disclaimer

**This is still a prototype, not fit for production yet. The API is still subject to breaking change, without warning.
Use at our own risk!**

**Even if it is still a prototype, it's still open to issue, idea, any input.**

**This is my pet project to learn rust, so there is, and there will probably be a lot of bad code, but I'm here to learn
so if you want to highlight something, you're more than welcome!**

## Api specification

You can find an openapi specification [here](crates/api/static/openapi.yml)

## Configuration

All the configuration must be pass through environment variables.

* `DATABASE_URL` (required): The URL to the postgres database
  as `postgres://POSTGRES_USER:POSTGRES_PASSWORD@HOST:5432/rss-aggregator`
* `REDIS_URL`: The redis URL as `redis://HOST`. Default `redis://locahost`
* `JWT_SECRET` (required): String used as the key for JWT
* `RSS_AGGREGATOR_ALLOW_ACCOUNT_CREATION` true/false (default false): Allow user to register an account. Otherwise, an
  admin should do it
* `POLLING_INTERVAL`: The number of seconds between feeds update. Default `300`
* `JAEGER_ENABLED`: If set to any value, enabled the Jaeger telemetry layer. Default `not set`
* `OTEL_EXPORTER_JAEGER_AGENT_HOST`: Hostname/IP of the [jaeger](https://www.jaegertracing.io/) agent.
  Default `localhost`
* `OTEL_EXPORTER_JAEGER_AGENT_PORT`: Port of the jaeger agent. Default `6831`
* `RUST_LOG`: (error/warn/info/debug/trace) Log level. Default `info`
* `SENTRY_DSN`: Your [sentry](https://sentry.io/welcome/) DSN if you have one. If not provided, disable sentry
* `FAILURE_THRESHOLD`: Number of failure before automatically disabling a channel. If 0, never disable it. Default `3`
* `FETCH_TIMEOUT`: Timeout in seconds for RSS feed fetching. Default `3`
* `RATE_LIMITING_BUCKET_SIZE`: Set quota size that defines how many requests can occur before the governor middleware
  starts blocking requests from an IP address and clients have to wait until the elements of the quota are replenished.
  Default `100`
* `RATE_LIMITING_FILL_RATE`: Set the interval after which one element of the quota is replenished in seconds.
  Default `10`

## What does it use

* `sqlx` for migration
* `ActixWeb` for, well, the web stuff
* `Tracing/Jaeger` for logs and observability

## How to launch it quickly

### Using docker-compose

```shell
docker compose up
```

This will create the databases (postgres + redis), do the necessary migrations and launch the jaeger burrito

## How to init/migrate database

Requires rust and cargo

```shell
cargo install sqlx-cli --no-default-features --features native-tls,postgres
# Time to take a coffee brake ☕
DATABASE_URL="postgres://rss-aggregator:rss-aggregator@localhost/rss-aggregator" sqlx migrate run
```
