# Lib Codec Package

The purpose of this package is to prove out generating C bindings for a `serde_cbor` based encode/decode library. You can use it as a jumping off point
for developing your own Rust based encoder/decoder library. Not only can it be used as-is in Rust but you can also export to a C library/header for Arm targets (thumbv8m for the nRF9160 Feather). That way it can be used in an embedded context without rolling your own library!

## Installing GNU Arm Toolchain

On OSX you can install using:

```
# tap the repository
$ brew tap osx-cross/arm

# install the toolchain
$ brew install arm-gcc-bin
```

Make sure that `GNUARMEMB_TOOLCHAIN_PATH` is then pointing to your toolchain directory. As an example:

```
export GNUARMEMB_TOOLCHAIN_PATH=/path/to/gcc-arm-none-eabi-X-YYYY-qZ-major
```

## Generating header

```
cd lib-codec-example
cbindgen --config cbindgen.toml --crate pyrinas-codec-example --output generated/libpyrinas_codec_example.h --lang c
```

## Generating library

In order for things to work on ARM, 

```
cargo build --package pyrinas-codec-example --target thumbv8m.main-none-eabihf --release --no-default-features
cp ../target/thumbv8m.main-none-eabihf/release/libpyrinas_codec_example.a generated/
```

The results will be stored in `/target/thumbv8m.main-none-eabihf/release` as `libpyrinas_codec_example.a`

## Using the library

Code size is important. The optimized library uses about 3000 bytes of space which is fairly on par with other serialize/deserialize solutions:

**Without:**

```
[181/191] Linking C executable zephyr/zephyr_prebuilt.elf
Memory region         Used Size  Region Size  %age Used
           FLASH:      153664 B     441856 B     34.78%
            SRAM:       33112 B       128 KB     25.26%
        IDT_LIST:         120 B         2 KB      5.86%
[191/191] Generating zephyr/merged.hex
```

**With:**

[180/189] Linking C executable zephyr/zephyr_prebuilt.elf
Memory region         Used Size  Region Size  %age Used
           FLASH:      156592 B     441856 B     35.44%
            SRAM:       33112 B       128 KB     25.26%
        IDT_LIST:         120 B         2 KB      5.86%
[189/189] Generating zephyr/merged.hex

156592-153664 = **2928 bytes**! nooooooot bad

One major drawback in all of this was the need to add and link the library ignoring 
the multiple definition error. 

```cmake
# Add external Rust lib directory
set(pyrinas_codec_example_dir   ${CMAKE_CURRENT_SOURCE_DIR}/lib)
set(pyrinas_codec_example_include_dir   ${CMAKE_CURRENT_SOURCE_DIR}/lib/include)

# Add the library
add_library(pyrinas_codec_example_lib STATIC IMPORTED GLOBAL)

# Set the paths
set_target_properties(pyrinas_codec_example_lib PROPERTIES IMPORTED_LOCATION             ${pyrinas_codec_example_dir}/libpyrinas_codec_example.a)
set_target_properties(pyrinas_codec_example_lib PROPERTIES INTERFACE_INCLUDE_DIRECTORIES ${pyrinas_codec_example_include_dir})

# Link them!
target_link_libraries(app PUBLIC pyrinas_codec_example_lib -Wl,--allow-multiple-definition)
```

Without arguments in the last line above I would get some nasty linker output:

```
/opt/nordic/ncs/v1.4.1/toolchain/bin/../lib/gcc/arm-none-eabi/9.2.1/../../../../arm-none-eabi/bin/ld: ../lib/libpyrinas_codec_example.a(compiler_builtins-4b08bafba0311fb4.compiler_builtins.9p5qppc2-cgu.149.rcgu.o): in function `__aeabi_ldivmod':
/cargo/registry/src/github.com-1ecc6299db9ec823/compiler_builtins-0.1.36/src/arm.rs:109: multiple definition of `__aeabi_ldivmod'; /opt/nordic/ncs/v1.4.1/toolchain/bin/../lib/gcc/arm-none-eabi/9.2.1/thumb/v8-m.main+fp/hard/libgcc.a(_aeabi_ldivmod.o):(.text+0x0): first defined here
/opt/nordic/ncs/v1.4.1/toolchain/bin/../lib/gcc/arm-none-eabi/9.2.1/../../../../arm-none-eabi/bin/ld: ../lib/libpyrinas_codec_example.a(compiler_builtins-4b08bafba0311fb4.compiler_builtins.9p5qppc2-cgu.149.rcgu.o): in function `__aeabi_uldivmod':
/cargo/registry/src/github.com-1ecc6299db9ec823/compiler_builtins-0.1.36/src/arm.rs:43: multiple definition of `__aeabi_uldivmod'; /opt/nordic/ncs/v1.4.1/toolchain/bin/../lib/gcc/arm-none-eabi/9.2.1/thumb/v8-m.main+fp/hard/libgcc.a(_aeabi_uldivmod.o):(.text+0x0): first defined here
collect2: error: ld returned 1 exit status
```

The above is complaining about some floating point functions that made themselves into the library despite using the `#[no_builtins]` macro.


## Don't want to generate anything

I've pre-generated the header and library and placed it in the `generated` folder. Simply include the static library and header file in your Zephyr project and you'll be off to the races.

## More reading

If you'd like to read more about exporting Rust libraries for use in C code check out this great article on the [Interrupt Blog.](https://interrupt.memfault.com/blog/rust-for-digital-signal-processing#building-a-rust-library) Also for more info on `cbindgen` you can check out [this article](https://karroffel.gitlab.io/post/2019-05-15-rust/) as well. After searching I also found [this one](https://justjjy.com/Rust-no-std) which discusses condtionally making a crate `std`/`no_std`. That way it can be used both in a `std` contexta and `no_std` for embedded!