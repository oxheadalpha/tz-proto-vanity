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
# download protocol file from Tezos node
export TEZOS_NODE=http://localhost:8732

# list known protocols
curl $TEZOS_NODE/protocols | jq

# choose protocol and download it
export TEZOS_PROTO=PtLimaPtLMwfNinJi9rCfDPWea8dFgTZ1MeJ9f1m2SRic6ayiwW
curl -L -H "Accept: application/octet-stream" $TEZOS_NODE/protocols/$TEZOS_PROTO > my.proto

# start vanity hash generator
tz-proto-vanity my.proto PtMumb

```

For more details, see [How to submit a Tezos Protocol Proposal](https://medium.com/the-aleph/how-to-submit-a-tezos-protocol-proposal-1704d3b73b8e).

Usage:

```
Usage: tz-proto-vanity [OPTIONS] <proto_file> <vanity_string>

Arguments:
  <proto_file>     Path to Tezos protocol source file.
  <vanity_string>  Look for protocol hashes starting with this string, e.g. PtMumbai

Options:
  -i, --ignore-case                    perform case insensitive matching
  -j, --thread-count <thread_count>    number of threads to use (default: determine automatically based on the number of available cores/CPUs)
  -f, --output-format <output_format>  Output format [default: human] [possible values: human, csv]
  -h, --help                           Print help
  -V, --version                        Print version
```
