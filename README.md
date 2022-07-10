# RSS Aggregator

RSS, but in rust

## How to launch it

## What does it use

* `diesel` for the database migration since I have difficulties with SeaORM migration
* `SeaORM` for async database stuff :)
* `ActixWeb`

## Using docker-compose

```shell
docker compose up
```

This will create the databases (postgres + redis) and do the necessary migrations

## How to init/migrate database

### Using docker

```shell
docker run --rm \
    -v "$PWD:/volume" \
    -w /volume \
    -e "DATABASE_URL=postgres://rss-aggregator:rss-aggregator@localhost/rss-aggregator" \
    -it clux/diesel-cli diesel migration run
```

### Using diesel cli directly

Requires rust and cargo 

```shell
cargo install diesel_cli --no-default-features --features postgres
# Time to take a coffee brake ☕
DATABASE_URL="postgres://rss-aggregator:rss-aggregator@localhost/rss-aggregator" diesel migration run
```

## How to (re)build entities

Database migrate must be already done.

```shell
cargo install sea-orm-cli # if needed
DATABASE_URL="postgres://rss-aggregator:rss-aggregator@localhost/rss-aggregator" sea-orm-cli generate entity -o entity/src --with-serde both --expanded-format
```

Cf. SeaORM [documentation](https://www.sea-ql.org/SeaORM/docs/generate-entity/sea-orm-cli)
