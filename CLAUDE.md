# colcon-cargo-ros2: Dual-Workspace ROS 2 Build System

**This repository contains two separate Cargo workspaces:**
- **user-libs**: User-facing libraries (`rclrs`, `rosidl-runtime-rs`) - requires ROS 2 environment
- **build-tools**: Build infrastructure (`rosidl-parser`, `rosidl-codegen`, `rosidl-bindgen`, `cargo-ros2`, `colcon-cargo-ros2`) - no ROS required

## Repository Structure

```
colcon-cargo-ros2/  (THIS REPOSITORY)
├── .envrc                        # Automatic ROS 2 environment setup (direnv)
├── CLAUDE.md                     # This file (project instructions)
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
├── build-tools/
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
└── # BUILD & CI
    ├── .github/workflows/        # CI for both workspaces
    ├── .gitignore                # Rust + Python patterns
    └── docs/                     # Architecture and design docs
```

## Development Workflows

### Environment Setup

**Automatic (Recommended)**:
```bash
# Install direnv: https://direnv.net/
direnv allow  # Once - permits .envrc to run

# ROS 2 environment now automatically loads when entering directory
cd colcon-cargo-ros2/  # ✅ Sourced ROS 2 jazzy environment
```

**Manual**:
```bash
source /opt/ros/jazzy/setup.bash  # Or humble, iron, etc.
```

### Build-Tools Workspace (No ROS Required)

```bash
# Build build-tools workspace
just build-build-tools

# Test build-tools workspace
just test-build-tools

# Clean build-tools workspace
just clean-build-tools

# Or use cargo directly
cd build-tools && cargo build --workspace
```

### User-Libs Workspace (Requires ROS 2)

```bash
# Ensure ROS is sourced first!
source /opt/ros/jazzy/setup.bash  # Or use direnv

# Build user-libs workspace
just build-user-libs

# Test user-libs workspace
just test-user-libs

# Clean user-libs workspace
just clean-user-libs

# Or use cargo directly
cd user-libs && cargo build --workspace
```

### Python Package (colcon-cargo-ros2)

```bash
# Build Python wheel (includes Rust extension via maturin)
just build-python

# Install wheel
just install

# Or install in development mode (editable)
just install-python
```

### Combined Commands

```bash
# Build everything (both workspaces + Python wheel)
just build

# Test everything
just test

# Format all code
just format

# Lint all code
just check

# Run full quality checks (format + lint + test)
just quality

# Clean everything
just clean
```

---

# cargo-ros2: All-in-One ROS 2 Rust Build Tool

## Project Overview

**cargo-ros2** is a next-generation, unified build tool for ROS 2 Rust projects that solves the fundamental circular dependency problem in current ros2_rust implementations **and** provides complete ament-compatible installation.

**Core Innovation**:
1. **Pre-build**: Project-local binding generation in `target/ros2_bindings/` with automatic Cargo patch management
2. **Post-build**: Complete ament layout installation (absorbing cargo-ament-build functionality)

**📖 See `docs/UNIFIED_ARCHITECTURE.md` for the complete architectural design.**

## The Problem We're Solving

### Current ros2_rust Issues

1. **Circular Dependency**:
   - Cargo.toml requires `vision_msgs = "*"`
   - Cargo queries crates.io → finds yanked version
   - .cargo/config.toml patch points to `install/vision_msgs/.../rust/`
   - But that path doesn't exist until after build starts
   - **Result**: Build fails unless interface packages are pre-built in workspace

2. **System Package Incompatibility**:
   - `ros-humble-vision-msgs` (apt) only has C/C++/Python bindings
   - No Rust bindings in `/opt/ros/humble/`
   - User must manually build interface packages locally
   - Requires colcon workspace with specific build order

3. **Workspace Requirement**:
   - Can't build standalone Rust ROS projects
   - Must use colcon with 3-stage build (ros2_rust → interface → packages)
   - Complex setup for simple projects

### Our Solution

**Unified all-in-one build tool** approach:
1. **Pre-build**: `cargo ros2` intercepts build process
2. Discovers ROS packages via `ament_index` (works with system packages!)
3. Generates Rust bindings to `target/ros2_bindings/<pkg>/`
4. Auto-configures `.cargo/config.toml` patches
5. **Build**: Invokes standard `cargo build` (or `check` for pure libs)
6. **Post-build**: Installs to ament layout (replaces cargo-ament-build)
   - Binaries → `install/<pkg>/lib/<pkg>/`
   - Source → `install/<pkg>/share/<pkg>/rust/`
   - Markers → `share/ament_index/resource_index/`
7. Success - ready for colcon!

## Architecture

### Two-Tool Design

**cargo-ros2** is split into two complementary tools:

1. **`cargo-ros2-bindgen`** - Low-level binding generator
   - Generates Rust bindings for a single ROS interface package
   - Can be used standalone
   - MVP: Shells out to Python `rosidl_generator_rs`

2. **`cargo-ros2`** - High-level build orchestrator
   - Three-phase workflow: generate bindings → build → install
   - Discovers ROS dependencies from Cargo.toml
   - Manages cache, patches .cargo/config.toml
   - Absorbs cargo-ament-build functionality

### Directory Structure

**Standalone Package**:
```
my_robot_project/
├── Cargo.toml                    # Standard manifest
├── .cargo/
│   └── config.toml               # Auto-generated patches
├── target/
│   ├── ros2_bindings/            # Package-local bindings
│   │   ├── vision_msgs/
│   │   │   ├── Cargo.toml
│   │   │   ├── src/lib.rs        # FFI bindings
│   │   │   └── build.rs          # Links C libs
│   │   └── sensor_msgs/
│   ├── debug/
│   └── release/
├── .ros2_bindgen_cache           # Metadata (checksums, timestamps)
└── src/
    └── main.rs
```

**Colcon Workspace** (automatically detected):
```
workspace/
├── build/
│   ├── ros2_bindings/            # Workspace-level shared bindings
│   │   ├── std_msgs/             # Generated once, shared by all packages
│   │   ├── geometry_msgs/
│   │   └── robot_interfaces/
│   └── .ros2_bindgen_cache       # Workspace cache
├── install/
│   ├── robot_controller/
│   └── robot_driver/
└── src/
    ├── robot_controller/
    │   ├── Cargo.toml
    │   ├── .cargo/config.toml    # Points to ../../build/ros2_bindings/
    │   └── src/main.rs
    └── robot_driver/
        ├── Cargo.toml
        ├── .cargo/config.toml    # Also points to ../../build/ros2_bindings/
        └── src/main.rs
```

**Key Feature**: Workspace-level bindings eliminate duplication when multiple packages use the same ROS messages.

### User Workflow

```bash
# 1. Create project (standard Cargo)
cargo new my_robot && cd my_robot

# 2. Add ROS dependencies (standard Cargo.toml)
[dependencies]
rclrs = "0.4"
vision_msgs = "*"
sensor_msgs = "*"

# 3. Build with wrapper (all magic happens here)
cargo ros2 build

# Behind the scenes:
# - Discovers vision_msgs via ament_index (apt-installed package!)
# - Generates to target/ros2_bindings/vision_msgs/
# - Patches .cargo/config.toml
# - Runs cargo build
# - Success!
```

## Key Design Principles

1. **Project-Isolated**: Bindings in `target/` → no global state, `cargo clean` works
2. **Zero Configuration**: User writes normal Cargo.toml, wrapper handles everything
3. **System Package Compatible**: Discovers ROS packages via `ament_index`
4. **Standard Cargo Experience**: Patches are transparent, normal deps in Cargo.toml
5. **Incremental**: Smart caching avoids regeneration (checksum-based)
6. **colcon-Friendly**: Drop-in replacement for current cargo invocations

## Recent Architectural Improvements (2025-11-11)

### Dual Workspace Architecture (2025-11-11)

Split the project into two independent Cargo workspaces to separate concerns:

**Problem**: Building `rclrs` requires ROS 2 environment (`ROS_DISTRO`, sourced setup.bash), which made it impossible to run `cargo check` on build tools without ROS installed.

**Solution**: Separate user-facing libraries from build infrastructure:

```
user-libs/          # Requires ROS 2 environment to build
├── rclrs/          # ROS 2 client library (Node, Publisher, Subscription)
└── rosidl-runtime-rs/  # Runtime support (Message trait, Sequence, String)

build-tools/        # No ROS required - can develop without ROS installed!
├── rosidl-parser/      # IDL parsing logic
├── rosidl-codegen/     # Code generation with templates
├── rosidl-bindgen/     # Embeds user-libs at compile time
├── cargo-ros2/         # Build orchestrator
└── colcon-cargo-ros2/  # Python/PyO3 colcon extension
```

**Benefits**:
- **Independent development**: Can work on build tools without ROS environment
- **Faster CI**: Build tools workspace checks don't require ROS installation
- **Clear separation**: User-facing APIs separate from build infrastructure
- **Embedded user-libs**: `rosidl-bindgen` embeds `rclrs` and `rosidl-runtime-rs` at compile time using `include_dir!` macro

**New justfile commands**:
```bash
just build-build-tools   # Build without ROS
just build-user-libs     # Build with ROS (requires sourced environment)
just build               # Build both workspaces
```

**Environment setup**: Added `.envrc` for automatic ROS sourcing via [direnv](https://direnv.net/)

---

## Previous Architectural Improvements (2025-11-07)

### Shared Runtime Library (`rosidl_runtime_rs`)

Eliminated 100+ line stub modules duplicated across every generated package by creating a single shared runtime library:

**Key Components**:
1. **FFI Layer** (`ffi.rs`): Raw C bindings to `rosidl_runtime_c`
   - String operations (init, fini, assign, copy, are_equal)
   - Primitive sequence operations for all types (f32, f64, i8-i64, u8-u64, bool)

2. **Idiomatic API** (`string.rs`, `sequence.rs`): Safe Rust wrappers
   - Automatic memory management via Drop
   - Conversions to/from Rust std types
   - Clone and PartialEq implementations

3. **Core Traits** (`traits.rs`):
   - `SequenceElement`: Type relationships (idiomatic ↔ RMW)
   - `SequenceAlloc`: Message-specific sequence operations
   - `Message`, `RmwMessage`, `Service`, `Action`: Core ROS type traits

**Benefits**:
- **Code reuse**: Single implementation shared by all packages
- **Maintainability**: One place to fix bugs and add features
- **Smaller binaries**: No duplicate implementations

### Workspace-Level Shared Bindings

Implemented workspace-level binding generation to eliminate duplication in colcon workspaces:

**Problem**: Multiple packages in a workspace would each generate their own copies of ROS message bindings:
```
src/pkg1/target/ros2_bindings/std_msgs/      # Duplicate
src/pkg2/target/ros2_bindings/std_msgs/      # Duplicate
```

**Solution**: Automatic workspace detection generates bindings once at workspace level:
```
build/ros2_bindings/std_msgs/                # Generated once
build/ros2_bindings/geometry_msgs/           # Shared by all packages
src/pkg1/.cargo/config.toml → ../../build/ros2_bindings/*
src/pkg2/.cargo/config.toml → ../../build/ros2_bindings/*
```

**How it works**:
1. **Automatic Detection**: Walks up directory tree looking for `build/` or `install/` directories
2. **Workspace Context**: Creates `WorkflowContext::new_workspace_level()` for shared output
3. **Unified Patches**: All packages get Cargo patches pointing to `build/ros2_bindings/`
4. **Fallback**: Standalone packages still use `target/ros2_bindings/` (no colcon required)

**Benefits**:
- **No duplication**: std_msgs generated once, not per-package
- **Faster builds**: Less codegen, less compilation
- **Smaller workspace**: Eliminates hundreds of MB of duplicate code
- **Follows ROS conventions**: Mirrors Python's approach (single site-packages per package, shared via PYTHONPATH)

**Cleanup**:
- `colcon clean` → removes `build/ros2_bindings/`
- `cargo clean` (in package) → removes package's `target/` only

### Workspace-Aware Library Linking

Fixed linker errors by making `build.rs` search workspace-local `install/` directories:

```rust
// Walks up directory tree to find colcon workspace root
if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
    for _ in 0..10 {
        let install_dir = search_dir.join("install");
        if install_dir.exists() {
            // Add all install/*/lib directories to linker search path
            for entry in std::fs::read_dir(&install_dir) {
                let lib_path = entry.path().join("lib");
                println!("cargo:rustc-link-search=native={}", lib_path.display());
            }
        }
    }
}
```

**Impact**: Packages can now find custom interface libraries built earlier in the colcon workspace, eliminating "library not found" linker errors.

### Profile-Aware Installation

Fixed binary installation by passing build profile (debug/release) to installer:

**Before**: Hardcoded `target/release`, causing failures for debug builds
**After**: Uses `target/{profile}` based on `--release` flag

```rust
let profile = if release { "release" } else { "debug" };
let target_dir = self.project_root.join("target").join(&self.profile);
```

### colcon Integration Fix

Fixed false-failure reporting in `colcon-ros-cargo` build task:

**Problem**: Returned `CompletedProcess` object instead of integer exit code
**Fix**: Extract `.returncode` from result before returning

```python
result = await run(self.context, cmd, cwd=pkg_path, env=None)
return result.returncode if result else 0  # colcon expects int, not object
```

**Result**: colcon now correctly reports "Finished" instead of "Failed" for successful builds.

## Implementation Status

**Current Phase**: Phase 2 Complete! ✅

**Completed**:
- ✅ **Phase 0**: Project Preparation (3/3 subphases)
- ✅ **Phase 1**: Native Rust IDL Generator (6/6 subphases)
  - Full rosidl parser (messages, services, actions)
  - Askama-based code generator
  - FFI bindings and runtime traits
  - 80 tests passing
- ✅ **Phase 2**: cargo-ros2 Tools (2/2 subphases)
  - **cargo-ros2-bindgen**: Standalone binding generator (13 tests)
  - **cargo-ros2**: Complete build workflow with caching (26 tests)
  - 151 total tests passing
- ✅ **Phase 3**: Production Features (14/20 subphases)
  - **rosidl_runtime_rs**: Shared runtime library with FFI bindings
  - **Workspace-aware build.rs**: Finds local install/ libraries
  - **Profile-aware installation**: Handles debug/release builds correctly
  - **colcon integration**: Fixed false-failure reporting
  - **Complete ament-build workflow**: Generate → Build → Install

**What Works Now**:
- Generate Rust bindings for any ROS 2 interface package
- Discover packages from system installation and workspace
- Intelligent SHA256-based caching
- Automatic .cargo/config.toml patching
- Complete CLI: `build`, `check`, `clean`, `ament-build`
- Full colcon integration with proper exit codes
- Installation to ament layout with markers and hooks

**Next**: Phase 3 completion (error handling, enhanced docs) - See `docs/ROADMAP.md`.

## Quick Reference

### Commands

**cargo-ros2-bindgen** (standalone tool):
```bash
# Generate bindings for a single package
cargo-ros2-bindgen --package std_msgs --output target/ros2_bindings

# With verbose output
cargo-ros2-bindgen --package geometry_msgs --output ./bindings --verbose

# Using direct path (bypasses ament index)
cargo-ros2-bindgen --package my_msgs --output ./out --package-path /path/to/share
```

**cargo-ros2** (main build tool):
```bash
cargo ros2 build                        # ✅ Build with binding generation
cargo ros2 build --verbose              # ✅ Verbose output
cargo ros2 build --bindings-only        # ✅ Generate bindings only (no build)

cargo ros2 check                        # ✅ Fast check (reuses bindings)
cargo ros2 check --bindings-only        # ✅ Generate bindings only (no check)

cargo ros2 clean                        # ✅ Clean bindings + cache

cargo ros2 ament-build --install-base <path>  # ✅ Generate + build + install to ament layout
cargo ros2 ament-build --install-base <path> --release  # ✅ Release build with installation
```

**Future enhancements**:
```bash
cargo ros2 test                         # Test with bindings
cargo ros2 cache --list                 # Show cached bindings
cargo ros2 cache --rebuild              # Force regeneration
```

### Key Files

**Primary Documentation** (minimalist style):
- `CLAUDE.md` - This file (project instructions)
- `docs/ARCH.md` - **⭐ Architecture overview (two-tool design)**
- `docs/DESIGN.md` - Implementation details
- `docs/ROADMAP.md` - Phase-by-phase implementation plan

**Reference**:
- `external/` - ros2_rust packages for reference (rosidl_rust, rosidl_runtime_rs, etc.)
- `tmp/` - Cloned repos for analysis (cargo-ament-build, colcon-ros-cargo)
- `docs/archive/` - Verbose design docs (historical)

## Development Guidelines

### Temporary Files

**Important**: All temporary files and directories (downloads, clones, analysis artifacts) should be created in `$PROJECT_ROOT/tmp/`. This directory is gitignored and keeps the workspace clean.

```bash
# Use project-local tmp directory
mkdir -p tmp/
cd tmp/
# ... work with temporary files ...
```

### File Manipulation Tools

**CRITICAL RULE: ALWAYS use Write/Edit tools for file operations**

**NEVER use Bash commands for file I/O**:
- ❌ **NEVER**: `cat > file`, `cat <<EOF`, `echo > file`, or any shell redirection
- ✅ **ALWAYS**: Use `Write` tool to create new files
- ✅ **ALWAYS**: Use `Edit` tool to modify existing files
- ✅ **ALWAYS**: Use `Read` tool to view file contents

**Rationale**:
- Claude Code's file tools provide better error handling and validation
- Bash commands like `cat` and `echo` are prone to quoting/escaping issues
- Write/Edit tools integrate properly with the Claude Code ecosystem
- Clearer intent and easier to track file modifications

**Example**:
```
# ❌ NEVER do this:
Bash: cat > tmp/test.rs <<'EOF'
fn main() { println!("test"); }
EOF

Bash: echo "content" >> file.txt

# ✅ ALWAYS do this instead:
Write: tmp/test.rs
Content: fn main() { println!("test"); }

Edit: existing_file.txt
old_string: old content
new_string: new content
```

**Exception**: Only use Bash for actual system commands (git, cargo, npm, make, colcon, etc.), never for file I/O operations.

### Temporary Files and Scripts

**CRITICAL RULE: All temporary files MUST be created in `$PROJECT_ROOT/tmp/` using Write/Edit tools**

**For ALL temporary files** (scripts, test data, build artifacts, analysis output):
- ✅ **ALWAYS**: Use `Write` tool to create files in `tmp/`
- ✅ **ALWAYS**: Use `Edit` tool to modify files in `tmp/`
- ❌ **NEVER**: Use bash redirects (`>`, `>>`, `cat <<EOF`, etc.) for temp files
- ❌ **NEVER**: Create temp files outside `tmp/` directory

**Rationale**:
- Keeps workspace clean and organized
- `tmp/` is gitignored by default
- Write/Edit tools provide better error handling
- Easier to track what files are created
- Temp files can be referenced later if needed

**Example**:
```
# ✅ CORRECT:
Write: tmp/test_data.json
Content: {"key": "value"}

Write: tmp/build_script.sh
Content: |
  #!/bin/bash
  cargo build --release

Bash: chmod +x tmp/build_script.sh && tmp/build_script.sh

# ❌ WRONG:
Bash: echo '{"key": "value"}' > /tmp/test_data.json
Bash: cat > build_script.sh <<'EOF'
  #!/bin/bash
  cargo build --release
EOF
```

### Temporary Python Scripts

**IMPORTANT**: When executing Python code for testing, exploration, or validation, ALWAYS create temporary scripts in `$PROJECT_ROOT/tmp/` instead of using inline code execution.

**NEVER use inline Python execution** (`python3 -c "..."` or heredocs):
- ❌ **NEVER**: `python3 << 'EOF' ... EOF`
- ❌ **NEVER**: `python3 -c "import sys; ..."`
- ✅ **ALWAYS**: Create `.py` file in `tmp/`, then execute it

**Rationale**:
- Better visibility of what code is being executed
- Easier to debug and iterate on scripts
- Scripts can be reused or referenced later
- Avoids complex quoting/escaping issues with inline code
- Clearer separation between commands and test scripts

**Example**:
```bash
# ❌ NEVER do this:
python3 -c "import sys; print(sys.path)"

python3 << 'EOF'
import json
data = json.load(sys.stdin)
EOF

# ✅ ALWAYS do this instead:
# 1. Create script using Write tool
Write: tmp/check_python_path.py
Content: |
  #!/usr/bin/env python3
  import sys
  print("Python path:")
  for p in sys.path:
      print(f"  {p}")

# 2. Execute the script
Bash: python3 tmp/check_python_path.py
```

**Benefits**:
- Scripts remain in `tmp/` for reference
- Easy to modify and re-run
- Can be version controlled if valuable
- `tmp/` is gitignored by default

### Build and Install Workflow

**CRITICAL**: After modifying code or templates, you MUST rebuild and reinstall to see changes take effect.

#### Template Changes (Most Important!)

Askama embeds templates at compile time. Simply rebuilding isn't enough - you must **clean and reinstall**:

```bash
# REQUIRED after modifying .jinja templates
just clean-build-tools   # Clear build cache (forces template re-embedding)
just build-python        # Rebuild wheel (includes cargo-ros2 + rosidl-bindgen)
just install             # Install wheel with all tools
```

**Why this matters**:
- Templates in `build-tools/rosidl-codegen/templates/*.jinja` are embedded into `rosidl-bindgen` at compile time
- Without cleaning, Askama may use cached template artifacts
- The Python wheel bundles the Rust tools, so `just build-python` rebuilds everything
- Without reinstalling, the old wheel continues to be used

#### Code Changes (Less Critical)

For regular code changes (not templates):

```bash
just build-python    # Rebuild wheel
just install         # Install updated wheel
```

#### Quick Development Cycle

```bash
# 1. Make changes to code/templates
# 2. Clean and rebuild (if templates changed)
just clean-build-tools
just build-python
just install

# 3. Test changes in workspace
cd testing_workspaces/complex_workspace
rm -rf build/ros2_bindings build/.colcon src/*/.cargo/config.toml
colcon build --packages-select <your-package>

# 4. Verify results
# If templates still don't apply, double-check you ran `just clean-build-tools`!
```

**Common Mistake**: Forgetting to rebuild the wheel after making changes. The Python wheel bundles the Rust binaries, so changes won't take effect until you run `just build-python && just install`.

### Code Quality

**IMPORTANT**: Always run format and lint before finishing your work:

```bash
just quality      # Format + lint + test (REQUIRED before committing)
```

This ensures:
- Code is consistently formatted with nightly rustfmt (both workspaces)
- All clippy warnings are fixed (treated as errors with `-D warnings`)
- All tests pass (Rust + Python)
- Zero warnings in the codebase

Alternative commands:
```bash
just format               # Format code only (both workspaces + Python)
just check                # Lint only (both workspaces + Python)
just test                 # Test only (both workspaces + Python)

# Workspace-specific commands
just build-build-tools    # Build build-tools workspace only
just test-build-tools     # Test build-tools workspace only (no ROS required)
just build-user-libs      # Build user-libs workspace (requires ROS)
just test-user-libs       # Test user-libs workspace (requires ROS)
```

**Best Practice**: Run `just quality` at the end of each work session or before marking tasks as complete. This catches issues early and maintains high code quality standards.

### Documentation

- Keep DESIGN.md up-to-date with architecture changes
- Update ROADMAP.md when completing phases
- Add examples for each feature
- Document edge cases and limitations

### Testing Strategy

1. **Unit tests**: Core binding generation logic
2. **Integration tests**: Full workflow with mock ROS packages
3. **Real-world tests**: Test with actual ROS 2 installations (Humble, Iron, Jazzy)
4. **Regression tests**: Known issues from ros2_rust (yanked deps, etc.)

## Related Projects

- **ros2_rust**: Current official Rust bindings (workspace-based approach)
- **r2r**: Alternative bindings (build.rs generation, slower builds)
- **cargo-ament-build**: Installs Cargo artifacts in ament layout (**being absorbed into cargo-ros2**)
- **colcon-ros-cargo**: Build plugin we'll integrate with (and potentially fork/modify)

## License

MIT OR Apache-2.0 (to be decided - compatible with ROS 2 ecosystem)

## Contributing

(To be added once project is public)

---

**Status**: Phase 3 Near Complete - Production Features (2025-11-11)
**Progress**: 14/20 subphases (70%) | 190+ tests passing | Zero warnings
**Latest**: Dual workspace architecture ✅, Embedded user-libs ✅, .envrc for auto ROS sourcing ✅
**Architecture**: Two independent workspaces (user-libs + build-tools), workspace-level binding generation, complete colcon integration
**Next**: Phase 3.4 - Enhanced Testing & Documentation, then Phase 4 - colcon Integration (see docs/ROADMAP.md)
