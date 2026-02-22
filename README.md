# Installation

Install this with `cargo install --path=.`

# Usage

1. Download the dump from https://mcsr-downloads.mrderp.dev/
1. Initalize a postgresql 18 database using `cat init.sql | psql db_name`
1. Run the converter with `ranked-db-converter [jsonl dump path]`

Default dump path: `dump.jsonl`
