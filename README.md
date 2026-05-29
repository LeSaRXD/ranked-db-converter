# Installation

Install this with `cargo install --path=.`

# Usage

1. Download the dump from https://mcsr-downloads.mrderp.dev/
1. Initalize a postgresql 18 database using `cat init.sql | psql db_name`
1. Run the converter with `DATABASE_URL='postgres://username@localhost/db_name' ranked-db-converter [jsonl dump path]`

Default dump path: `dump.jsonl`

# Migrating db from 0.1.0

Run the following in the database

```sql
alter table elo_change rename new_elo to old_elo;
alter table player alter username type varchar;
```
