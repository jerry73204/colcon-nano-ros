# cargo-nano-ros

`cargo nano-ros` — Cargo subcommand front-end for [nano-ros](https://github.com/NEWSLabNTU/nano-ros) message generation. Reads `package.xml`, resolves transitive ROS 2 dependencies from the ament index, and emits a tree of generated Rust crates under `generated/` with a `.cargo/config.toml` `[patch.crates-io]` block wired to them.

```bash
cargo install cargo-nano-ros
cargo nano-ros generate --force
```

Subcommands: `generate-rust`, `generate-c`, `generate-cpp`. Internally this crate is a thin shim over `nros-cli-core`; users on machines without Cargo can install the `nros` CLI for the same surface.

## License

Licensed under either of [Apache-2.0](https://www.apache.org/licenses/LICENSE-2.0) or [MIT](https://opensource.org/licenses/MIT) at your option.

Part of the [nano-ros](https://github.com/NEWSLabNTU/nano-ros) project.
