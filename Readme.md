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


## Generating OTA Manifest file

In order to generate a manifest file you'll need `deno` installed. On OSX run:

```
brew install deno
```

For other platforms see the [documentation](https://deno.land/#installation).

Then, in your Pyrinas firmware directory run:

```
> deno run --allow-run --allow-write manifest-generator.ts
```

It will spit out some results for your review:

```
❯ deno run --allow-run --allow-write manifest-generator.ts
Check file:///.../manifest-generator.ts
version 0.1.0-6-g74b7376
manifest generated!
{
  version: {
    major: 0,
    minor: 1,
    patch: 0,
    commit: 6,
    hash: [
      103, 55, 52, 98,
       55, 51, 55, 54
    ]
  },
  file: "app_update.bin",
  force: false
}
File written to ./manifest.json
```

## License

This repository has an Apache 2.0 license. Contributions will be licensed the same
with no additional terms or conditions. See `LICEENSE` file for more information.
