# Licensed under the Apache License, Version 2.0

"""Workspace-level ROS 2 binding generation for Rust.

This module provides centralized binding generation for an entire colcon workspace.
Instead of each package generating bindings independently (causing race conditions),
this module generates ALL bindings once before any packages are built.

Architecture:
1. Discover all ROS package dependencies in the workspace
2. Generate all bindings to build/ros2_bindings/
3. Detect Cargo workspace(s) and write .cargo/config.toml to proper locations
4. Individual packages then just run `cargo build`
"""

import subprocess
from pathlib import Path
from typing import Dict, List
import xml.etree.ElementTree as ET

from colcon_core.logging import colcon_logger

logger = colcon_logger.getChild(__name__)


class WorkspaceBindingGenerator:
    """Generates ROS 2 Rust bindings for an entire colcon workspace."""

    def __init__(
        self, workspace_root: Path, build_base: Path, install_base: Path, args
    ):
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
        3. Detects Cargo workspaces
        4. Writes .cargo/config.toml files
        """
        logger.info("Starting workspace-level binding generation")

        # Step 1: Discover all ROS dependencies from ament_index and workspace
        ros_packages = self._discover_ros_packages()
        logger.info(f"Discovered {len(ros_packages)} ROS packages")

        # Step 2: Generate bindings for all discovered packages
        self._generate_bindings(ros_packages, verbose)

        # Step 3: Detect Cargo workspaces in the colcon workspace
        cargo_workspaces = self._detect_cargo_workspaces()
        logger.info(f"Detected {len(cargo_workspaces)} Cargo workspace(s)")

        # Step 4: Write .cargo/config.toml for each Cargo workspace
        for cargo_ws in cargo_workspaces:
            self._write_cargo_config(cargo_ws, ros_packages)

        logger.info("Workspace-level binding generation complete")

    def _discover_ros_packages(self) -> Dict[str, Path]:
        """Discover all ROS packages from ament_index, install/, and workspace source.

        Returns:
            Dict mapping package names to their share/ directory paths
        """
        packages = {}

        # 1. Discover from ament_index (system packages + already installed workspace packages)
        try:
            from ament_index_python.packages import (
                get_package_share_directory,
                get_packages_with_prefixes,
            )

            all_packages = get_packages_with_prefixes()
            for pkg_name, pkg_prefix in all_packages.items():
                try:
                    pkg_share = Path(get_package_share_directory(pkg_name))
                    if pkg_share.exists() and (pkg_share / "package.xml").exists():
                        packages[pkg_name] = pkg_share
                except Exception:
                    continue
        except ImportError:
            logger.warning("ament_index_python not available")

        # 2. Check workspace install directory for packages not yet in ament_index
        if self.install_base.exists():
            for pkg_install in self.install_base.iterdir():
                if not pkg_install.is_dir():
                    continue
                share_dir = pkg_install / "share" / pkg_install.name
                if share_dir.exists() and (share_dir / "package.xml").exists():
                    packages[pkg_install.name] = share_dir

        # 3. Scan workspace source directories for ROS interface packages
        # This discovers workspace packages that haven't been installed yet
        for src_dir_name in ["src", "ros"]:
            src_dir = self.workspace_root / src_dir_name
            if not src_dir.exists():
                continue

            # Walk the source directory looking for package.xml files
            for package_xml in src_dir.rglob("package.xml"):
                pkg_path = package_xml.parent
                # Read package name from package.xml
                try:
                    tree = ET.parse(package_xml)
                    root = tree.getroot()
                    pkg_name_elem = root.find("name")
                    if pkg_name_elem is not None and pkg_name_elem.text:
                        pkg_name = pkg_name_elem.text.strip()
                        # Only add if not already discovered and has interfaces
                        if pkg_name not in packages:
                            has_interfaces = any(
                                [
                                    (pkg_path / "msg").exists(),
                                    (pkg_path / "srv").exists(),
                                    (pkg_path / "action").exists(),
                                ]
                            )
                            if has_interfaces:
                                packages[pkg_name] = pkg_path
                                logger.debug(
                                    f"Discovered workspace package: {pkg_name} at {pkg_path}"
                                )
                except Exception as e:
                    logger.debug(f"Failed to parse {package_xml}: {e}")

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
            binding_dir = self.bindings_dir / pkg_name
            if binding_dir.exists():
                # TODO: Add checksum-based cache validation
                logger.debug(f"Bindings already exist for {pkg_name}")
                continue

            # Generate bindings using cargo-ros2-bindgen
            logger.info(f"Generating bindings for {pkg_name}")
            self._run_bindgen(pkg_name, pkg_share, binding_dir, verbose)

            # Post-process Cargo.toml to remove path dependencies
            self._fixup_cargo_toml(pkg_name, binding_dir)

    def _run_bindgen(
        self, pkg_name: str, pkg_share: Path, output_dir: Path, verbose: bool
    ):
        """Run cargo-ros2-bindgen to generate bindings for a single package.

        Args:
            pkg_name: Name of the ROS package
            pkg_share: Path to the package's share/ directory
            output_dir: Path where bindings should be generated
            verbose: Enable verbose output
        """
        cmd = [
            "cargo-ros2-bindgen",
            "--package",
            pkg_name,
            "--package-path",
            str(pkg_share),
            "--output",
            str(output_dir),
        ]

        if verbose:
            cmd.append("--verbose")

        try:
            result = subprocess.run(cmd, capture_output=True, text=True, check=True)
            if verbose:
                logger.info(f"Bindgen output: {result.stdout}")
        except subprocess.CalledProcessError as e:
            logger.error(f"Failed to generate bindings for {pkg_name}:")
            logger.error(f"  Command: {' '.join(cmd)}")
            logger.error(f"  stdout: {e.stdout}")
            logger.error(f"  stderr: {e.stderr}")
            raise

    def _fixup_cargo_toml(self, pkg_name: str, binding_dir: Path):
        """Post-process Cargo.toml to convert path dependencies to version requirements.

        This is necessary because cargo-ros2-bindgen generates bindings with local
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
                logger.warning(f"No Cargo.toml found for {pkg_name}")
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

    def _detect_cargo_workspaces(self) -> List[Path]:
        """Detect all Cargo workspaces in the colcon workspace.

        Returns:
            List of paths to Cargo workspace roots (directories containing Cargo.toml with [workspace])
        """
        cargo_workspaces = []

        # Search in common source directories
        for src_dir in [self.workspace_root / "src", self.workspace_root / "ros"]:
            if not src_dir.exists():
                continue

            # Walk the directory tree looking for Cargo.toml files
            for cargo_toml in src_dir.rglob("Cargo.toml"):
                # Read and check if it contains [workspace] section
                try:
                    content = cargo_toml.read_text()
                    if "[workspace]" in content:
                        cargo_ws_root = cargo_toml.parent
                        if cargo_ws_root not in cargo_workspaces:
                            cargo_workspaces.append(cargo_ws_root)
                            logger.info(f"Found Cargo workspace: {cargo_ws_root}")
                except Exception as e:
                    logger.warning(f"Failed to read {cargo_toml}: {e}")

        # If no Cargo workspaces found, each package is standalone
        # In this case, we'll need to write config to each package directory
        if not cargo_workspaces:
            logger.info("No Cargo workspaces found, will use per-package configs")
            cargo_workspaces = self._find_standalone_packages()

        return cargo_workspaces

    def _find_standalone_packages(self) -> List[Path]:
        """Find standalone Cargo packages (those not in a workspace).

        Returns:
            List of paths to standalone package roots
        """
        standalone_packages = []

        for src_dir in [self.workspace_root / "src", self.workspace_root / "ros"]:
            if not src_dir.exists():
                continue

            for cargo_toml in src_dir.rglob("Cargo.toml"):
                content = cargo_toml.read_text()
                # If it's not a workspace and not a member of a workspace
                if "[workspace]" not in content:
                    pkg_root = cargo_toml.parent
                    # Check if parent is a workspace member
                    if not self._is_workspace_member(pkg_root):
                        standalone_packages.append(pkg_root)

        return standalone_packages

    def _is_workspace_member(self, pkg_path: Path) -> bool:
        """Check if a package is a member of a Cargo workspace.

        Args:
            pkg_path: Path to the package directory

        Returns:
            True if the package is a workspace member, False otherwise
        """
        # Walk up the directory tree looking for a workspace
        current = pkg_path.parent
        while current != self.workspace_root and current != current.parent:
            workspace_toml = current / "Cargo.toml"
            if workspace_toml.exists():
                content = workspace_toml.read_text()
                if "[workspace]" in content:
                    # Check if pkg_path is in the workspace members
                    # For now, assume yes if workspace exists above
                    return True
            current = current.parent
        return False

    def _write_cargo_config(self, cargo_ws_root: Path, ros_packages: Dict[str, Path]):
        """Write .cargo/config.toml for a Cargo workspace.

        Args:
            cargo_ws_root: Path to the Cargo workspace root
            ros_packages: Dict of all ROS packages (for building patch entries)
        """
        config_dir = cargo_ws_root / ".cargo"
        config_file = config_dir / "config.toml"

        # Create .cargo directory
        config_dir.mkdir(parents=True, exist_ok=True)

        # Build [patch.crates-io] section
        patches = []
        rosidl_runtime_rs_path = None

        for pkg_name in sorted(ros_packages.keys()):
            binding_dir = self.bindings_dir / pkg_name
            if binding_dir.exists():
                # cargo-ros2-bindgen creates nested structure: pkg_name/pkg_name/Cargo.toml
                # Check if the nested package directory exists
                nested_pkg_dir = binding_dir / pkg_name
                if nested_pkg_dir.exists() and (nested_pkg_dir / "Cargo.toml").exists():
                    # Use the nested package directory
                    patches.append(
                        f'{pkg_name} = {{ path = "{nested_pkg_dir.absolute()}" }}'
                    )
                elif (binding_dir / "Cargo.toml").exists():
                    # Use the top-level directory if Cargo.toml is there
                    patches.append(
                        f'{pkg_name} = {{ path = "{binding_dir.absolute()}" }}'
                    )

                # Save the first rosidl_runtime_rs path we find
                if rosidl_runtime_rs_path is None:
                    runtime_rs = binding_dir / "rosidl_runtime_rs"
                    if runtime_rs.exists() and (runtime_rs / "Cargo.toml").exists():
                        rosidl_runtime_rs_path = runtime_rs.absolute()

        # Add rosidl_runtime_rs patch (shared across all packages)
        if rosidl_runtime_rs_path:
            patches.append(
                f'rosidl_runtime_rs = {{ path = "{rosidl_runtime_rs_path}" }}'
            )

        # Write config.toml
        content = "[patch.crates-io]\n"
        content += "\n".join(patches)
        content += "\n\n"

        config_file.write_text(content)
        logger.info(
            f"Wrote .cargo/config.toml with {len(patches)} patches to {config_file}"
        )


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
    generator = WorkspaceBindingGenerator(
        workspace_root, build_base, install_base, args
    )

    # Only generate if we're the first process to get the lock
    if generator.should_generate():
        generator.generate_all_bindings(verbose)
    else:
        logger.info("Binding generation already handled by another process")
