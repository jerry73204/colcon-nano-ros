# colcon-cargo-ros2
Build Rust ROS 2 projects with colcon using cargo-ros2.

This colcon extension integrates [cargo-ros2](https://github.com/your-org/cargo-ros2) as the build tool for Rust packages in ROS 2 workspaces. cargo-ros2 automatically discovers ROS dependencies, generates Rust bindings for ROS messages/services/actions, and handles ament-compatible installation.

## Prerequisites

You need to install cargo-ros2:

```bash
cargo install cargo-ros2
```

## Usage

Packages need to have a `package.xml` in addition to `Cargo.toml`. You should see such packages classified as `ament_cargo` in the output of `colcon list`. If they are classified as `ros.cargo_ros2` instead, the `colcon-cargo-ros2` extension has not been found by `colcon`. Make sure that you have installed the extension (`pip install .` in this directory).

Simply list dependencies (other `ament_cargo` packages or message packages) in `Cargo.toml` and `package.xml` as if they were hosted on crates.io. cargo-ros2 will:
- Discover ROS dependencies via ament_index
- Generate Rust bindings automatically
- Cache generated bindings for fast rebuilds
- Create a `.cargo/config.toml` file with proper patches
- Install to ament-compatible locations

Extra arguments to `cargo` can be passed via the `--cargo-args` option, e.g. `colcon build --cargo-args --release`.

After building, run binaries with `ros2 run`.

You can also use `cargo ros2` directly as the primary build tool. The `.cargo/config.toml` file and bindings cache work seamlessly whether you use `colcon` or `cargo ros2 build` directly. When the dependency graph changes, run `cargo ros2 cache rebuild` or rebuild with `colcon`.


## Features

- **Automatic Binding Generation**: cargo-ros2 generates Rust bindings for all ROS message/service/action types on-demand
- **Smart Caching**: Generated bindings are cached with SHA256 checksums for fast incremental builds
- **Parallel Generation**: Multiple packages generate bindings in parallel using rayon
- **Ament Compatible**: Installs to standard ament locations for seamless ROS 2 integration
- **Progress Indicators**: Beautiful progress bars show generation and build status

## Limitations

- `colcon test` is not yet fully supported (use `cargo test` directly)
- The quadratic build cost issue (Cargo rebuilding dependencies) is mitigated but not eliminated by cargo-ros2's caching system