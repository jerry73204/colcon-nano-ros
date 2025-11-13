# Combined Rust + Python justfile for colcon-cargo-ros2

# Default recipe - show available commands
default:
    @just --list

# === BUILD-TOOLS WORKSPACE ===

# Build build-tools workspace
build-build-tools:
    #!/usr/bin/env bash
    set -e
    cd build-tools
    cargo build \
        --profile dev-release \
        --all-targets

# Test build-tools workspace
test-build-tools:
    #!/usr/bin/env bash
    set -e
    cd build-tools
    cargo nextest run \
        --cargo-profile dev-release \
        --no-fail-fast

# Format build-tools workspace
format-build-tools:
    #!/usr/bin/env bash
    set -e
    cd build-tools
    cargo +nightly fmt

# Check/lint build-tools workspace
check-build-tools:
    #!/usr/bin/env bash
    set -e
    cd build-tools
    cargo clippy --workspace --all-targets -- -D warnings

# Clean build-tools workspace
clean-build-tools:
    #!/usr/bin/env bash
    set -e
    cd build-tools
    cargo clean
    rm -rf colcon-cargo-ros2/dist/ colcon-cargo-ros2/build/ colcon-cargo-ros2/*.egg-info

# === USER-LIBS WORKSPACE (requires ROS environment) ===

# Build user-libs workspace
build-user-libs:
    #!/usr/bin/env bash
    for crate in user-libs/*; do
        (cd $crate && \
        cargo build --all-targets)
    done

# Test user-libs workspace
test-user-libs:
    #!/usr/bin/env bash
    for crate in user-libs/*; do
        (cd $crate && \
        cargo nextest run --no-fail-fast)
    done

# Clean user-libs workspace
clean-user-libs:
    #!/usr/bin/env bash
    set -e
    cd user-libs
    cargo clean

# === PYTHON COMMANDS ===

# Build Python package (wheel)
build-python:
    #!/usr/bin/env bash
    set -e
    cd build-tools/colcon-cargo-ros2
    maturin build --profile dev-release

# Install Python package in development mode
install-python:
    pip3 install -e build-tools/colcon-cargo-ros2/ --break-system-packages

# Test Python code
test-python:
    pytest build-tools/colcon-cargo-ros2/test/

# Format Python code
format-python:
    #!/usr/bin/env bash
    set -e
    cd build-tools/colcon-cargo-ros2
    ruff format colcon_cargo_ros2/ test/

# Lint Python code
check-python:
    #!/usr/bin/env bash
    set -e
    cd build-tools/colcon-cargo-ros2
    ruff check colcon_cargo_ros2/ test/

# === COMBINED COMMANDS ===

# Build both workspaces (note: user-libs requires ROS environment)
build: build-build-tools build-user-libs

# Test both workspaces + Python
test: test-build-tools test-user-libs test-python

# Clean both workspaces
clean: clean-build-tools clean-user-libs

# Format all code (both workspaces + Python)
format:
    just format-build-tools
    just format-python

# Lint and check all code (both workspaces + Python)
check:
    just check-build-tools
    just check-python

# === QUALITY COMMANDS ===

# Run all quality checks (format, lint, test)
quality: format check test
