# colcon-cargo-ros2

**Build Rust ROS 2 packages with automatic message binding generation.**

`colcon-cargo-ros2` is a colcon extension that enables seamless integration of Rust packages in ROS 2 workspaces. It automatically generates Rust bindings for ROS message types, manages dependencies, and installs packages in ament-compatible layout.

## Features

- **Automatic Binding Generation**: Generates Rust bindings for messages, services, and actions on-demand
- **Smart Caching**: SHA256-based checksums for fast incremental builds
- **Workspace-Level Bindings**: Bindings generated once and shared across all packages
- **Zero Configuration**: Just add dependencies to `Cargo.toml` - bindings are handled automatically
- **Ament Compatible**: Installs to standard ament locations for seamless ROS 2 integration

## Installation

### From PyPI (Recommended)

```bash
pip install colcon-cargo-ros2
```

### From Source

```bash
git clone https://github.com/jerry73204/colcon-cargo-ros2.git
cd colcon-cargo-ros2
pip install packages/colcon-cargo-ros2/
```

## Quick Start

### 1. Create a ROS 2 Workspace

```bash
mkdir -p ~/ros2_ws/src
cd ~/ros2_ws
```

### 2. Create a Rust ROS 2 Package

```bash
cd src
cargo new --bin my_robot_node
cd my_robot_node
```

### 3. Add ROS Dependencies

**Cargo.toml**:
```toml
[package]
name = "my_robot_node"
version = "0.1.0"
edition = "2021"

[dependencies]
rclrs = "0.6"
std_msgs = "*"
geometry_msgs = "*"
```

**package.xml**:
```xml
<?xml version="1.0"?>
<?xml-model href="http://download.ros.org/schema/package_format3.xsd" schematypens="http://www.w3.org/2001/XMLSchema"?>
<package format="3">
  <name>my_robot_node</name>
  <version>0.1.0</version>
  <description>Example Rust ROS 2 node</description>
  <maintainer email="you@example.com">Your Name</maintainer>
  <license>Apache-2.0</license>

  <buildtool_depend>ament_cmake</buildtool_depend>

  <depend>rclrs</depend>
  <depend>std_msgs</depend>
  <depend>geometry_msgs</depend>

  <export>
    <build_type>ament_cargo</build_type>
  </export>
</package>
```

**src/main.rs**:
```rust
use rclrs::{Context, Node, RclrsError};
use std_msgs::msg::String as StringMsg;

fn main() -> Result<(), RclrsError> {
    let context = Context::new(std::env::args())?;
    let node = Node::new(&context, "my_robot_node")?;

    let publisher = node.create_publisher::<StringMsg>("chatter", 10)?;

    let mut count = 0;
    loop {
        let mut msg = StringMsg::default();
        msg.data = format!("Hello from Rust! {}", count);
        publisher.publish(msg)?;

        println!("Published: {}", count);
        count += 1;

        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
```

### 4. Build with colcon

```bash
cd ~/ros2_ws
source /opt/ros/jazzy/setup.bash  # Or your ROS 2 distro
colcon build --symlink-install
```

The extension will:
1. Discover ROS dependencies from `Cargo.toml` and `package.xml`
2. Generate Rust bindings for `std_msgs` and `geometry_msgs`
3. Build your Rust package with cargo
4. Install binaries to `install/my_robot_node/lib/my_robot_node/`

### 5. Run Your Node

```bash
source install/setup.bash
ros2 run my_robot_node my_robot_node
```

## Package Structure

For `colcon-cargo-ros2` to recognize your package:
- **Both files required**: `package.xml` AND `Cargo.toml` in the package root
- **Build type**: `package.xml` must specify `<build_type>ament_cargo</build_type>` in the `<export>` section
- **Dependencies**: List ROS dependencies in both `Cargo.toml` and `package.xml`

Verify packages are detected:
```bash
$ colcon list
my_robot_node   src/my_robot_node   (ament_cargo)
```

## Building

### Basic Commands

```bash
# Build all packages
colcon build

# Build specific package
colcon build --packages-select my_robot_node

# Build with release optimizations
colcon build --cargo-args --release

# Verbose output
colcon build --event-handlers console_direct+
```

### Using Custom Interfaces

Custom interface packages follow the standard ROS 2 procedure (CMake-based with `rosidl_generate_interfaces`). Simply add them as dependencies in your Rust package's `Cargo.toml`:

```toml
[dependencies]
my_custom_interfaces = "*"
```

Bindings will be generated automatically during the build.

## How It Works

### Workspace-Level Binding Generation

When building a colcon workspace, `colcon-cargo-ros2`:

1. **Discovers Packages**: Finds all ROS dependencies via ament index
2. **Generates Bindings**: Creates Rust bindings in `build/ros2_bindings/`
3. **Configures Cargo**: Updates each package's `.cargo/config.toml` with patches
4. **Builds**: Runs `cargo build` with workspace-level config
5. **Installs**: Copies binaries and creates ament markers

**Workspace Structure**:
```
ros2_ws/
├── build/
│   └── ros2_bindings/          # Shared bindings (generated once)
│       ├── std_msgs/
│       ├── geometry_msgs/
│       └── my_interfaces/
├── install/
│   ├── my_robot_node/
│   │   ├── lib/my_robot_node/  # Binaries
│   │   └── share/              # Metadata
│   └── my_interfaces/
└── src/
    ├── my_robot_node/
    │   ├── Cargo.toml
    │   ├── package.xml
    │   └── .cargo/config.toml  # Auto-generated patches
    └── my_interfaces/
```

### Benefits

- **No Duplication**: `std_msgs` generated once, not per-package
- **Fast Builds**: Intelligent caching skips regeneration when possible
- **Clean Workspace**: `colcon clean` removes all generated code
- **Standard Cargo**: Normal Cargo workflows work as expected

## Troubleshooting

### "Package not found in ament index"

Make sure the ROS 2 environment is sourced:
```bash
source /opt/ros/jazzy/setup.bash
```

### "error: failed to select a version"

This usually means bindings weren't generated. Try:
```bash
# Clean and rebuild
rm -rf build install
colcon build
```

### Build fails with linking errors

Ensure all dependencies are listed in both `Cargo.toml` and `package.xml`:
```xml
<depend>std_msgs</depend>
<depend>geometry_msgs</depend>
```

## Requirements

- **Python**: 3.8 or later
- **ROS 2**: Humble, Iron, or Jazzy
- **Rust**: 1.70 or later (stable toolchain)
- **colcon**: Latest version

## License

Apache-2.0 (compatible with ROS 2 ecosystem)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, architecture details, and guidelines.

## Related Projects

- [ros2_rust](https://github.com/ros2-rust/ros2_rust) - Official Rust bindings for ROS 2
- [r2r](https://github.com/sequenceplanner/r2r) - Alternative Rust bindings
- [colcon](https://colcon.readthedocs.io) - Build tool for ROS 2

## Support

- **Issues**: [GitHub Issues](https://github.com/jerry73204/colcon-cargo-ros2/issues)
- **Discussions**: [GitHub Discussions](https://github.com/jerry73204/colcon-cargo-ros2/discussions)
