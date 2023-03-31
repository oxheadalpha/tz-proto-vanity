This is a tool to generate vanity protocol hashes for Tezos.
It finds nonces so that when placed in protocol's source code,
protocol hash starts with the desired vanity string.

To build, [install
Rust](https://doc.rust-lang.org/cargo/getting-started/installation.html),
then

```sh
cargo build --release
```

Run it like so:

```sh
tz-proto-vanity ~/dev/tezos/src/proto_016_PtMumbai/lib_protocol/main.ml PtMumb
```

Usage:

```
Usage: tz-proto-vanity [OPTIONS] <proto_file> <vanity_string>

Arguments:
  <proto_file>     Path to Tezos protocol source file.
  <vanity_string>  Look for protocol hashes starting with this string (ignoring case by default), e.g. PtMumbai

Options:
  -e, --exact                        match vanity string exactly
  -j, --thread-count <thread_count>  number of threads to use (default: determine automatically based on the number of available cores/CPUs)
  -h, --help                         Print help
  -V, --version                      Print version
```
