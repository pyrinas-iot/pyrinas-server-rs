# Pyrinas Server

## Building Development Release

```
cargo build
```

Result will be placed in `target/debug`

## Building Production Release

```
cargo build --release
```

The release will be placed in `target/release`. As of this writing
the bin is called `pyrinas-server`.


## .env file

Currently the `.env` file defines what the server does and how it operates. An example can be found in `.env.sample`

## Set the log level for `env_logger`


The `pyrinas-server` example uses `env_logger`. In order to see output make sure the `RUST_LOG`  environment variable is set

```
$ export RUST_LOG=info
```
