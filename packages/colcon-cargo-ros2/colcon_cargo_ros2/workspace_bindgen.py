# Licensed under the Apache License, Version 2.0

"""Workspace-level ROS 2 binding generation for Rust.

This module provides centralized binding generation for an entire colcon workspace.
Instead of each package generating bindings independently (causing race conditions),
this module generates ALL bindings once before any packages are built.

Architecture:
1. Discover all ROS package dependencies in the workspace
2. Generate all bindings to build/ros2_bindings/
3. Write single Cargo config file in build/ros2_cargo_config.toml
4. Individual packages run `cargo build --config build/ros2_cargo_config.toml`
"""

from pathlib import Path
from typing import Dict

from colcon_core.logging import colcon_logger

# Import Rust library directly via PyO3 bindings
from colcon_cargo_ros2 import cargo_ros2_py

logger = colcon_logger.getChild(__name__)


class WorkspaceBindingGenerator:
    """Generates ROS 2 Rust bindings for an entire colcon workspace."""

    def __init__(self, workspace_root: Path, build_base: Path, install_base: Path, args):
        """Initialize the workspace binding generator.

        Args:
            workspace_root: Root directory of the colcon workspace
            build_base: Base directory for build artifacts (workspace/build/)
            install_base: Base directory for installed packages (workspace/install/)
            args: Colcon command line arguments
        """
        self.workspace_root = workspace_root
        self.build_base = build_base
        self.install_base = install_base
        self.args = args
        self.bindings_dir = build_base / "ros2_bindings"
        self.lock_file = build_base / ".colcon" / "bindgen.lock"

    def should_generate(self) -> bool:
        """Check if binding generation is needed (not already done by another process)."""
        # If lock file exists, another process is/was handling binding generation
        if self.lock_file.exists():
            logger.info(f"Binding generation lock exists: {self.lock_file}")
            return False

        # Create lock file to indicate we're handling binding generation
        self.lock_file.parent.mkdir(parents=True, exist_ok=True)
        self.lock_file.write_text("locked")
        return True

    def generate_all_bindings(self, verbose: bool = False):
        """Generate all ROS 2 bindings for the workspace.

        This is the main entry point that:
        1. Discovers all ROS dependencies
        2. Generates bindings for all packages
        3. Writes single Cargo config file in build/
        """
        logger.info("Starting workspace-level binding generation")

        # Step 1: Discover all ROS dependencies from ament_index and workspace
        ros_packages = self._discover_ros_packages()
        logger.info(f"Discovered {len(ros_packages)} ROS packages")

        # Step 2: Generate bindings for all discovered packages
        self._generate_bindings(ros_packages, verbose)

        # Step 3: Write single Cargo config file in build/ directory
        self._write_cargo_config_file(ros_packages)

        logger.info("Workspace-level binding generation complete")

    def _discover_ros_packages(self) -> Dict[str, Path]:
        """Discover all ROS packages from ament_index, install/, and colcon-discovered packages.

        Returns:
            Dict mapping package names to their share/ directory paths
        """
        packages = {}

        # 1. First priority: Use packages discovered by colcon's PackageAugmentationExtensionPoint
        # This respects colcon's package discovery (--base-paths, --packages-select, etc.)
        # and provides source .msg files instead of generated .idl files from install/
        from colcon_cargo_ros2.package_augmentation import RustBindingAugmentation

        interface_packages = getattr(RustBindingAugmentation, "_interface_packages", {})
        pkg_names = list(interface_packages.keys())
        logger.info(f"Colcon discovered {len(interface_packages)} interface packages: {pkg_names}")
        for pkg_name, pkg_path in interface_packages.items():
            # Use source directory directly for workspace packages
            packages[pkg_name] = pkg_path
            logger.info(f"Using colcon-discovered package: {pkg_name} at {pkg_path}")

        # 2. Discover from ament_index (system packages + already installed workspace packages)
        try:
            from ament_index_python.packages import (
                get_package_share_directory,
                get_packages_with_prefixes,
            )
        except ImportError as e:
            logger.error(
                f"\n\nament_index_python not found: {e}"
                "\n\nPlease install ROS 2 dependencies:"
                "\n  $ pip install ament_index_python\n"
            )
            raise

        all_packages = get_packages_with_prefixes()
        for pkg_name, pkg_prefix in all_packages.items():
            # Skip if already discovered from source (prioritize source over install)
            if pkg_name in packages:
                continue
            try:
                pkg_share = Path(get_package_share_directory(pkg_name))
                if pkg_share.exists() and (pkg_share / "package.xml").exists():
                    packages[pkg_name] = pkg_share
            except (LookupError, OSError) as e:
                # LookupError: Covers PackageNotFoundError and KeyError
                # OSError: Covers file system access issues
                logger.debug(f"Skipping package {pkg_name}: {e}")
                continue

        # 3. Check workspace install directory for packages not yet in ament_index
        if self.install_base.exists():
            for pkg_install in self.install_base.iterdir():
                if not pkg_install.is_dir():
                    continue
                # Skip if already discovered from source (prioritize source over install)
                if pkg_install.name in packages:
                    continue
                share_dir = pkg_install / "share" / pkg_install.name
                if share_dir.exists() and (share_dir / "package.xml").exists():
                    packages[pkg_install.name] = share_dir

        return packages

    def _generate_bindings(self, ros_packages: Dict[str, Path], verbose: bool):
        """Generate Rust bindings for all ROS packages.

        Args:
            ros_packages: Dict mapping package names to share/ directories
            verbose: Enable verbose output
        """
        # Create bindings output directory
        self.bindings_dir.mkdir(parents=True, exist_ok=True)

        # Generate bindings for each package that has interfaces
        for pkg_name, pkg_share in ros_packages.items():
            # Check if package has interfaces (msg/, srv/, action/ directories)
            has_interfaces = any(
                [
                    (pkg_share / "msg").exists(),
                    (pkg_share / "srv").exists(),
                    (pkg_share / "action").exists(),
                ]
            )

            if not has_interfaces:
                continue

            # Check if bindings already exist and are up-to-date
            binding_dir = self.bindings_dir / pkg_name / pkg_name
            if binding_dir.exists():
                # TODO: Add checksum-based cache validation
                logger.debug(f"Bindings already exist for {pkg_name}")
                continue

            # Generate bindings using cargo ros2 bindgen
            # Pass workspace-level bindings_dir, not package-specific dir
            # (generate_package will create the package subdirectory)
            logger.info(f"Generating bindings for {pkg_name}")
            try:
                self._run_bindgen(pkg_name, pkg_share, self.bindings_dir, verbose)
                # Post-process Cargo.toml to remove path dependencies
                self._fixup_cargo_toml(pkg_name, binding_dir)
            except RuntimeError as e:
                # Log warning for packages that can't be generated (e.g., unsupported IDL features)
                logger.warning(f"Skipping {pkg_name}: {e}")

    def _run_bindgen(self, pkg_name: str, pkg_share: Path, output_dir: Path, verbose: bool):
        """Generate Rust bindings for a single package using direct API call.

        Args:
            pkg_name: Name of the ROS package
            pkg_share: Path to the package's share/ directory
            output_dir: Path where bindings should be generated
            verbose: Enable verbose output
        """
        try:
            # Create configuration for binding generation
            config = cargo_ros2_py.BindgenConfig(
                package_name=pkg_name,
                output_dir=str(output_dir),
                package_path=str(pkg_share),
                verbose=verbose,
            )

            # Call Rust function directly (no subprocess!)
            cargo_ros2_py.generate_bindings(config)

            if verbose:
                logger.info(f"✓ Generated bindings for {pkg_name}")

        except RuntimeError as e:
            logger.error(f"Failed to generate bindings for {pkg_name}: {e}")
            raise

    def _fixup_cargo_toml(self, pkg_name: str, binding_dir: Path):
        """Post-process Cargo.toml to convert path dependencies to version requirements.

        This is necessary because cargo ros2 bindgen generates bindings with local
        path dependencies (e.g., `std_msgs = { path = "../std_msgs" }`), but we want
        to use the .cargo/config.toml patches instead.

        Args:
            pkg_name: Name of the package
            binding_dir: Directory containing the generated bindings
        """
        # Find the Cargo.toml (nested structure: binding_dir/pkg_name/Cargo.toml)
        cargo_toml = binding_dir / pkg_name / "Cargo.toml"
        if not cargo_toml.exists():
            # Try top-level
            cargo_toml = binding_dir / "Cargo.toml"
            if not cargo_toml.exists():
                # This is expected for packages without interfaces (msg/srv/action)
                logger.debug(f"No Cargo.toml found for {pkg_name} (package has no interfaces)")
                return

        # Read the Cargo.toml
        content = cargo_toml.read_text()
        lines = content.split("\n")

        # Process each line to convert path dependencies to version requirements
        new_lines = []
        in_dependencies = False
        for line in lines:
            # Track when we're in [dependencies] or [build-dependencies] section
            if line.strip().startswith("[dependencies]") or line.strip().startswith(
                "[build-dependencies]"
            ):
                in_dependencies = True
                new_lines.append(line)
                continue
            elif line.strip().startswith("[") and in_dependencies:
                in_dependencies = False
                new_lines.append(line)
                continue

            # If we're in dependencies section and line has a path dependency, convert it
            if in_dependencies and "{ path =" in line:
                # Extract package name from line like: `std_msgs = { path = "../std_msgs" }`
                if "=" in line:
                    dep_name = line.split("=")[0].strip()
                    # Convert all path dependencies to version requirements
                    # including rosidl_runtime_rs (will be patched to shared location)
                    new_lines.append(f'{dep_name} = "*"')
                    continue

            new_lines.append(line)

        # Write back the modified Cargo.toml
        cargo_toml.write_text("\n".join(new_lines))
        logger.debug(f"Fixed up Cargo.toml for {pkg_name}")

    def _write_cargo_config_file(self, ros_packages: Dict[str, Path]):
        """Write single Cargo config file in build/ directory.

        This config file will be passed to cargo via --config flag.

        Args:
            ros_packages: Dict of all ROS packages (for building patch entries)
        """
        config_file = self.build_base / "ros2_cargo_config.toml"

        # Build [patch.crates-io] section
        patches = []

        for pkg_name in sorted(ros_packages.keys()):
            binding_dir = self.bindings_dir / pkg_name
            if binding_dir.exists():
                # rosidl-bindgen creates nested structure: pkg_name/pkg_name/Cargo.toml
                # Check if the nested package directory exists
                nested_pkg_dir = binding_dir / pkg_name
                if nested_pkg_dir.exists() and (nested_pkg_dir / "Cargo.toml").exists():
                    # Use the nested package directory
                    patches.append(f'{pkg_name} = {{ path = "{nested_pkg_dir.absolute()}" }}')
                elif (binding_dir / "Cargo.toml").exists():
                    # Use the top-level directory if Cargo.toml is there
                    patches.append(f'{pkg_name} = {{ path = "{binding_dir.absolute()}" }}')

        # Add patches for embedded runtime libraries
        runtime_rs_dir = self.bindings_dir / "rosidl_runtime_rs"
        if runtime_rs_dir.exists() and (runtime_rs_dir / "Cargo.toml").exists():
            patches.append(f'rosidl_runtime_rs = {{ path = "{runtime_rs_dir.absolute()}" }}')

        rclrs_dir = self.bindings_dir / "rclrs"
        if rclrs_dir.exists() and (rclrs_dir / "Cargo.toml").exists():
            patches.append(f'rclrs = {{ path = "{rclrs_dir.absolute()}" }}')

        # Build [build] section with rustflags for linker search paths
        # This is critical for finding workspace-local ROS package libraries
        rustflags = []

        # Add workspace install directory lib paths
        if self.install_base.exists():
            for pkg_install in self.install_base.iterdir():
                if not pkg_install.is_dir():
                    continue
                lib_dir = pkg_install / "lib"
                if lib_dir.exists():
                    rustflags.append(f'"-L", "native={lib_dir.absolute()}"')

        # Add system ROS library paths from AMENT_PREFIX_PATH
        import os
        if "AMENT_PREFIX_PATH" in os.environ:
            for prefix in os.environ["AMENT_PREFIX_PATH"].split(":"):
                lib_path = Path(prefix) / "lib"
                if lib_path.exists():
                    rustflags.append(f'"-L", "native={lib_path.absolute()}"')

        # Write config.toml
        content = "[patch.crates-io]\n"
        content += "\n".join(patches)
        content += "\n"

        if rustflags:
            content += "\n[build]\n"
            content += "rustflags = [\n"
            content += ",\n".join(f"    {flag}" for flag in rustflags)
            content += "\n]\n"

        config_file.write_text(content)
        logger.info(f"Wrote Cargo config with {len(patches)} patches and {len(rustflags)} linker paths to {config_file}")


def generate_workspace_bindings(
    workspace_root: Path,
    build_base: Path,
    install_base: Path,
    args,
    verbose: bool = False,
):
    """Generate bindings for an entire workspace (convenience function).

    Args:
        workspace_root: Root directory of the colcon workspace
        build_base: Base directory for build artifacts
        install_base: Base directory for installed packages
        args: Colcon command line arguments
        verbose: Enable verbose output
    """
    generator = WorkspaceBindingGenerator(workspace_root, build_base, install_base, args)

    # Only generate if we're the first process to get the lock
    if generator.should_generate():
        generator.generate_all_bindings(verbose)
    else:
        logger.info("Binding generation already handled by another process")
