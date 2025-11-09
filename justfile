# Combined Rust + Python justfile for colcon-cargo-ros2

# Default recipe - show available commands
default:
    @just --list

# === RUST COMMANDS ===

# Build all Rust crates
build-rust:
    cargo build --workspace --profile dev-release --all-targets

# Test all Rust crates
test-rust:
    cargo test --workspace

# Lint Rust code
check-rust:
    cargo clippy --workspace --all-targets -- -D warnings

# Format Rust code
format-rust:
    cargo +nightly fmt

# === PYTHON COMMANDS ===

# Build Python package
build-python:
    cd colcon-cargo-ros2 && python3 -m build

# Install Python package in development mode
install-python:
    pip3 install -e colcon-cargo-ros2/ --break-system-packages

# Test Python code
test-python:
    pytest colcon-cargo-ros2/test/

# Format Python code
format-python:
    ruff format colcon-cargo-ros2/colcon_cargo_ros2/ colcon-cargo-ros2/test/

# Lint Python code
check-python:
    ruff check colcon-cargo-ros2/colcon_cargo_ros2/ colcon-cargo-ros2/test/

# === COMBINED COMMANDS ===

# Build everything (Rust only - Python doesn't need building for development)
build: build-rust

# Test everything (Rust + Python)
test: test-rust test-python

# Format everything (Rust + Python)
format: format-rust format-python

# Lint and check everything (Rust + Python)
check: check-rust check-python

# Install colcon-cargo-ros2 wheel (includes bundled cargo-ros2-py)
install:
    cd colcon-cargo-ros2 && maturin build --release
    pip3 install --force-reinstall colcon-cargo-ros2/target/wheels/colcon_cargo_ros2-*.whl --break-system-packages

# Clean all build artifacts
clean:
    cargo clean
    rm -rf colcon-cargo-ros2/target/
    rm -rf colcon-cargo-ros2/dist/ colcon-cargo-ros2/build/ colcon-cargo-ros2/*.egg-info

# === QUALITY COMMANDS ===

# Run all quality checks (format, lint, test)
quality: format check test
