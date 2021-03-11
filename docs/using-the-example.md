# Using the example

You'll need the latest version of `rust`. You can use [`rustup`](https://rustup.rs) to download and install.

```
> cargo build --package pyrinas-server
> cargo build --package pyrinas-cli
```

The server can be cross compiled using other platforms. Pyrinas is mostly used on and is compiled fairly regularly on FreeBSD. It does have some dependencies on OpenSSL/Libc so you'll need to configure these for your intended system of use.

This will build the example server and cli. They'll be available in the `target/debug/` folder. 