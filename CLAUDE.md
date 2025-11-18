# colcon-cargo-ros2: Development Guide

**Build Rust ROS 2 packages with automatic message binding generation.**

This file contains development instructions for Claude Code. User documentation is in [README.md](README.md).

## Repository Structure

```
colcon-cargo-ros2/  (THIS REPOSITORY)
├── .envrc                        # Automatic ROS 2 environment setup (direnv)
├── CLAUDE.md                     # This file (development instructions)
├── README.md                     # User-facing documentation
├── justfile                      # Build automation (dual workspace + Python)
│
├── # USER-FACING LIBRARIES (requires ROS 2)
├── user-libs/
│   ├── Cargo.toml                # Workspace manifest
│   ├── rclrs/                    # ROS 2 client library for Rust
│   └── rosidl-runtime-rs/        # Runtime library for ROS messages
│
├── # BUILD INFRASTRUCTURE (no ROS required)
├── packages/                     # Renamed from build-tools/
│   ├── Cargo.toml                # Workspace manifest
│   ├── rosidl-parser/            # ROS IDL parser (.msg, .srv, .action)
│   ├── rosidl-codegen/           # Code generator with Askama templates
│   ├── rosidl-bindgen/           # Binding generator (embeds user-libs)
│   ├── cargo-ros2/               # Build orchestrator (pre-build + post-build)
│   └── colcon-cargo-ros2/        # Python/PyO3 colcon extension
│       ├── colcon_cargo_ros2/    # Python module
│       ├── cargo-ros2-py/        # Rust library exposed to Python
│       └── test/                 # Python tests
│
└── .github/workflows/            # CI for both workspaces
```

## Development Workflows

### Environment Setup

**Automatic (Recommended)**:
```bash
# Install direnv: https://direnv.net/
direnv allow  # Once - permits .envrc to run
# ROS 2 environment now automatically loads when entering directory
```

**Manual**:
```bash
source /opt/ros/jazzy/setup.bash  # Or humble, iron, etc.
```

### Quick Build Commands

```bash
# Build everything (both workspaces + Python wheel)
just build

# Test everything
just test

# Format and lint all code
just format
just check

# Full quality check (format + lint + test)
just quality

# Clean everything
just clean
```

### Workspace-Specific Commands

```bash
# Build-tools/packages workspace (no ROS required)
just build-build-tools
just test-build-tools
just clean-build-tools

# User-libs workspace (requires ROS)
just build-user-libs
just test-user-libs
just clean-user-libs

# Python package
just build-python
just install-python
just install  # Install from wheel
```

### Version Management

```bash
# Bump version in both pyproject.toml and Cargo.toml
just bump-version 0.4.0

# Output:
# ✓ Updated packages/colcon-cargo-ros2/pyproject.toml
# ✓ Updated packages/colcon-cargo-ros2/Cargo.toml
# Next steps:
#   1. Review changes: git diff
#   2. Commit: git add -u && git commit -m 'Bump version to 0.4.0'
#   3. Tag: git tag v0.4.0
#   4. Build: just build-python
```

### Development Cycle

**CRITICAL**: After modifying code, rebuild and reinstall to see changes:

```bash
# 1. Make changes to code/templates

# 2. Clean and rebuild (REQUIRED for template changes)
just clean-build-tools   # Only needed if templates changed
just build-python        # Rebuild wheel (includes all Rust tools)
just install             # Install updated wheel

# 3. Test in a workspace
cd ~/test_workspace
rm -rf build/.colcon build/ros2_bindings
colcon build --packages-select <package>
```

**Why**: Templates are embedded at compile time. Python wheel bundles Rust binaries. Must reinstall to use updated tools.

## Key Development Guidelines

### File Manipulation Rules

**CRITICAL: ALWAYS use Write/Edit tools for file operations**

- ❌ **NEVER**: `cat > file`, `echo > file`, `cat <<EOF`, or any shell redirection
- ✅ **ALWAYS**: Use `Write` tool to create files
- ✅ **ALWAYS**: Use `Edit` tool to modify files
- ✅ **ALWAYS**: Use `Read` tool to view files

**Exception**: Bash is only for system commands (git, cargo, colcon, etc.), not file I/O.

### Temporary Files

**All temporary files MUST be created in `$PROJECT_ROOT/tmp/` using Write tool:**

```bash
# ✅ CORRECT
Write: tmp/test_data.json
Content: {"key": "value"}

Write: tmp/build_script.sh
Content: |
  #!/bin/bash
  cargo build --release

Bash: chmod +x tmp/build_script.sh && tmp/build_script.sh

# ❌ WRONG
Bash: echo '{"key": "value"}' > /tmp/test_data.json
Bash: cat > tmp/test.sh <<'EOF'
  #!/bin/bash
  ...
EOF
```

### Code Quality

**REQUIRED before committing**:
```bash
just quality  # Format + lint + test
```

Ensures:
- Code formatted with nightly rustfmt
- All clippy warnings fixed (`-D warnings`)
- All tests pass (Rust + Python)
- Zero warnings

## Recent Architectural Improvements

### Capitalized Boolean Literals Support (2025-11-18)

**Problem**: Parser failed on ROS 2 action files using capitalized boolean literals (`True`/`False`):
```
Failed to parse action: DockRobot: Unexpected token: expected Identifier, got string
```

**Root Cause**: Lexer only recognized `true`/`TRUE` and `false`/`FALSE`, but not `True`/`False`.

**Solution**: Extended lexer token definitions in `packages/rosidl-parser/src/lexer.rs`:
```rust
#[token("true")]
#[token("TRUE")]
#[token("True")]   // Added
True,

#[token("false")]
#[token("FALSE")]
#[token("False")]  // Added
False,
```

**Files Modified**:
- `packages/rosidl-parser/src/lexer.rs`

**Impact**: Successfully parses nav2_msgs actions (DockRobot, etc.) and other packages using capitalized booleans.

**Tests**: Added `lex_capitalized_boolean_literals` test.

---

### Action Constant Namespace Collision Fix (2025-11-18)

**Problem**: Action constants with duplicate names across Goal/Result/Feedback sections caused compilation errors:
```rust
error[E0428]: the name `NONE` is defined multiple times
  --> nav2_msgs/src/action/dock_robot_rmw.rs:32:1
   |
13 | pub const NONE: u16 = 0;  // Result section
32 | pub const NONE: u16 = 0;  // Feedback section
   | ^^^^^^^^^^^^^^^^^^^^^^^^ `NONE` redefined here
```

**Root Cause**: All constants were generated at module top level without namespace separation.

**Solution**: Wrapped constants in separate modules in `packages/rosidl-codegen/templates/action_rmw.rs.jinja`:
```rust
// Before (conflicting)
pub const NONE: u16 = 0;  // Result
pub const NONE: u16 = 0;  // Feedback (error!)

// After (namespaced)
pub mod result_constants {
    pub const NONE: u16 = 0;
}
pub mod feedback_constants {
    pub const NONE: u16 = 0;
}
```

**Usage**:
```rust
nav2_msgs::action::rmw::dock_robot::result_constants::NONE
nav2_msgs::action::rmw::dock_robot::feedback_constants::NONE
```

**Files Modified**:
- `packages/rosidl-codegen/templates/action_rmw.rs.jinja`

**Impact**: Enables actions with duplicate constant names (common in ROS 2 ecosystem).

---

### Package Version Extraction from package.xml (2025-11-18)

**Problem**: All generated message crates used hardcoded `ROSIDL_RUNTIME_RS_VERSION` ("0.5") instead of actual ROS package version.

**Solution**:
1. Added `version: String` field to `Package` struct
2. Implemented `parse_package_version()` to extract `<version>` from package.xml
3. Updated `generate_cargo_toml()` to use package version

**Example**:
```toml
# Before (incorrect)
[package]
name = "std_msgs"
version = "0.5.0"  # Wrong - used ROSIDL_RUNTIME_RS_VERSION

# After (correct)
[package]
name = "std_msgs"
version = "5.3.0"  # Correct - from package.xml
```

**Files Modified**:
- `packages/rosidl-bindgen/src/ament.rs`
- `packages/rosidl-bindgen/src/generator.rs`

**Impact**: Generated crates now have correct semantic versions matching ROS packages.

**Tests**: Added 6 new tests for version parsing and generation.

---

### Workspace-Local Library Linking Fix (2025-11-18)

**Problem**: Rust binaries failed to link against workspace-local ROS interface libraries:
```
rust-lld: error: unable to find library -lsplat_msgs__rosidl_typesupport_c
```

**Root Cause**: Cargo `build.rs` linker search paths (`cargo:rustc-link-search`) don't propagate to downstream binaries.

**Solution**: Added `[build]` rustflags to `ros2_cargo_config.toml`:
```toml
[build]
rustflags = [
    "-L", "native=/path/to/install/package/lib",
    "-L", "native=/opt/ros/jazzy/lib"
]
```

**Files Modified**: `packages/colcon-cargo-ros2/colcon_cargo_ros2/workspace_bindgen.py`

**Impact**: Enables workspaces with custom interface packages used by Rust packages.

---

### `[package.metadata.ros]` Installation Support (2025-11-17)

Implemented support for installing additional files (launch, config, URDF, etc.):

```toml
[package.metadata.ros]
install_to_share = ["launch", "config", "README.md"]  # Directories and files
install_to_include = ["include"]
install_to_lib = ["scripts"]
```

**Semantics**:
- **Directories**: Copied recursively with name preserved
- **Individual files**: Filename preserved (parent path dropped)
- **Missing paths**: Build fails with clear error

100% backward compatible with cargo-ament-build.

---

### WString Array/Sequence Support (2025-11-17)

Added complete WString support across all idiomatic templates to fix type mismatches in generated bindings.

---

### `--cargo-args` Support (2025-11-17)

Added ability to pass arguments to Cargo:
```bash
colcon build --cargo-args --release
colcon build --cargo-args --profile dev-release
```

---

### Ruff Linter Migration (2025-11-17)

Migrated from flake8 to ruff (10-100x faster, written in Rust, no plugin dependencies).

---

### Dual Workspace Architecture (2025-11-11)

Split into two independent workspaces:
- **user-libs/**: Requires ROS 2 environment (`rclrs`, `rosidl-runtime-rs`)
- **packages/**: No ROS required (build infrastructure)

Benefits:
- Can develop build tools without ROS installed
- Faster CI for build tools
- Clear separation of concerns

---

### Workspace-Level Shared Bindings (2025-11-07)

Bindings generated once at `build/ros2_bindings/`, shared by all packages:
```
build/ros2_bindings/std_msgs/      # Generated once
src/pkg1/.cargo/config.toml → ../../build/ros2_bindings/*
src/pkg2/.cargo/config.toml → ../../build/ros2_bindings/*
```

Benefits:
- No duplication (std_msgs generated once, not per-package)
- Faster builds
- Smaller workspace

## Testing

### Unit Tests

```bash
# All tests
just test

# Workspace-specific
just test-build-tools  # No ROS required
just test-user-libs    # Requires ROS

# Specific package
cd packages/rosidl-codegen && cargo test
```

### Integration Testing Workspaces

**testing_workspaces/complex_workspace** - Comprehensive message type testing:

Demonstrates 50+ message/action types from 17 different ROS 2 packages:

```bash
cd testing_workspaces/complex_workspace
just clean && just build  # Build workspace
just run                   # Execute test binary
```

**Coverage includes**:
- **Standard messages** (18 types): std_msgs, builtin_interfaces, geometry_msgs, sensor_msgs
- **Navigation** (5 types): nav_msgs (Odometry, Path, OccupancyGrid), trajectory_msgs
- **Control & Diagnostics** (4 types): control_msgs, diagnostic_msgs
- **Nav2 Actions** (6 types): NavigateToPose, DockRobot (tests capitalized booleans!)
- **Motion Planning** (3 types): moveit_msgs (RobotState, MotionPlanRequest, PlanningScene)
- **Action System** (3 types): action_msgs (GoalInfo, GoalStatus, GoalStatusArray)
- **Custom Interfaces** (6 types): robot_interfaces messages, services, actions

**Key Features Tested**:
- ✅ Capitalized boolean literals in action files
- ✅ Action constants in separate namespaces
- ✅ Complex nested message dependencies
- ✅ Custom interface packages
- ✅ Service and action type generation
- ✅ Workspace-level binding sharing

## CI/CD

GitHub Actions workflows:
- **wheels.yml**: Production builds (triggered by tags, publishes to PyPI)
- **test-build.yml**: Quick validation on PRs

Builds 31 artifacts (30 wheels + sdist) for Linux/macOS/Windows × Python 3.8-3.13.

## Status

**Version**: v0.3.2 (2025-11-18)
**Progress**: 15/20 subphases (75%) | 180 tests passing (177 Rust + 3 Python) | Zero warnings
**Latest**: Capitalized boolean support ✅, Action constant namespacing ✅, Package version extraction ✅
**Testing**: Validated with:
- autoware_carla_bridge (118 packages) ✅
- splat-drive workspace (6 packages + custom interfaces) ✅
- complex_workspace (50+ message types from 17 packages) ✅

**Versions**:
- Rust workspace: v0.2.0 (rosidl-parser, rosidl-codegen, rosidl-bindgen, cargo-ros2)
- Python package: v0.3.2 (colcon-cargo-ros2)
- Author: Lin Hsiang-Jui <jerry73204@gmail.com>

**PyPI**: 31 artifacts for Linux/macOS/Windows × Python 3.8-3.13

**Architecture**:
- Two independent workspaces (user-libs + packages)
- Workspace-level binding generation via colcon's package discovery
- Rustflags-based linker search paths for workspace libraries
- Complete ament layout installation
- Package versions extracted from package.xml

**Testing Coverage**:
- **Standard ROS 2 packages**: std_msgs, geometry_msgs, sensor_msgs, nav_msgs, trajectory_msgs, builtin_interfaces
- **Navigation**: nav2_msgs with complex actions (NavigateToPose, DockRobot with capitalized booleans)
- **Motion planning**: moveit_msgs with nested dependencies
- **Control**: control_msgs, diagnostic_msgs, action_msgs
- **Custom interfaces**: Messages, services, actions with full type support

**Key Features**:
- ✅ Capitalized boolean literals (`True`/`False`)
- ✅ Action constants in separate namespaces
- ✅ Correct package versions in generated crates
- ✅ Workspace-local library linking
- ✅ `[package.metadata.ros]` installation
- ✅ `--cargo-args` pass-through
- ✅ Version management with `just bump-version`

**Next**: Phase 3.4 - Enhanced Testing & Documentation
