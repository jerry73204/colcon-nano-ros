# nros

The `nros` command-line tool — the user-facing entry point to [nano-ros](https://github.com/NEWSLabNTU/nano-ros).

```bash
cargo install nros-cli

nros new my-project --platform freertos --rmw zenoh --lang c talker
nros generate rust
nros build
nros run
nros doctor
nros board list
```

Thin binary on top of `nros-cli-core`. Both this crate and the legacy `cargo nano-ros` subcommand route through the same library.

## License

Licensed under either of [Apache-2.0](https://www.apache.org/licenses/LICENSE-2.0) or [MIT](https://opensource.org/licenses/MIT) at your option.

Part of the [nano-ros](https://github.com/NEWSLabNTU/nano-ros) project.
