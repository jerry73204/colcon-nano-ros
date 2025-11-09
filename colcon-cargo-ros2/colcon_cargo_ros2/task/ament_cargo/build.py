# Licensed under the Apache License, Version 2.0

from pathlib import Path
import os
import subprocess

from colcon_core.logging import colcon_logger
from colcon_core.plugin_system import satisfies_version
from colcon_core.shell import create_environment_hook
from colcon_core.task import TaskExtensionPoint, run

from colcon_cargo_ros2.workspace_bindgen import generate_workspace_bindings


logger = colcon_logger.getChild(__name__)


class AmentCargoBuildTask(TaskExtensionPoint):
    """A build task for Rust ROS 2 packages using workspace-level binding generation.

    This task implements a two-phase approach:
    1. Workspace-level binding generation (done once before all builds)
    2. Per-package cargo build (just compiles, no binding generation)

    The workspace-level binding generation:
    - Discovers all ROS dependencies from ament_index and workspace
    - Generates ALL bindings to build/ros2_bindings/
    - Detects Cargo workspace(s) and writes .cargo/config.toml
    - Uses lock file to ensure only one process does generation

    This eliminates race conditions and improves build performance.
    """

    def __init__(self):  # noqa: D107
        super().__init__()
        satisfies_version(TaskExtensionPoint.EXTENSION_POINT_VERSION, "^1.0")

    def add_arguments(self, *, parser):  # noqa: D102
        # Note: --cargo-args is already defined by colcon core, so we don't redefine it
        pass

    async def build(self, *, additional_hooks=None):  # noqa: D102
        """Build the Rust ROS 2 package using workspace-level binding generation."""
        additional_hooks = [] if additional_hooks is None else additional_hooks

        # Step 1: Generate workspace-level bindings (done once for entire workspace)
        rc = await self._prepare_workspace_bindings()
        if rc:
            return rc

        # Step 2: Create environment hooks
        await self._create_environment_hooks(additional_hooks)

        # Step 3: Build this package with cargo
        args = self.context.args
        cmd = self._build_cmd(args.cargo_args if hasattr(args, "cargo_args") else [])

        # Execute cargo build
        result = await run(self.context, cmd, cwd=self.context.pkg.path, env=None)
        if result and result.returncode != 0:
            return result.returncode

        # Step 4: Install binaries and create package markers
        rc = self._install_package()
        if rc:
            return rc

        # Return the exit code
        return 0

    async def _prepare_workspace_bindings(self):
        """Generate workspace-level ROS 2 bindings (done once for entire workspace)."""
        # Check for cargo-ros2 binary
        try:
            result = subprocess.run(
                ["cargo", "ros2", "--help"], capture_output=True, check=True
            )
            logger.debug("cargo-ros2 found")
        except (subprocess.CalledProcessError, FileNotFoundError):
            logger.error(
                "\n\ncargo-ros2 not found!"
                "\n\nPlease ensure cargo-ros2 is installed:"
                "\n $ cargo install --path cargo-ros2\n"
            )
            return 1

        # Derive workspace paths from install_base
        args = self.context.args
        workspace_root = Path(os.path.abspath(os.path.join(args.install_base, "../..")))
        build_base = Path(os.path.abspath(os.path.join(args.build_base, "..")))
        install_base = Path(args.install_base).parent  # install/ directory

        # Generate workspace-level bindings
        # This uses a lock file, so only the first package will actually generate
        # All other packages will see the lock and skip generation
        try:
            verbose = getattr(args, "verbose", False)
            generate_workspace_bindings(
                workspace_root, build_base, install_base, args, verbose
            )
        except Exception as e:
            logger.error(f"Workspace binding generation failed: {e}")
            import traceback

            logger.error(traceback.format_exc())
            return 1

        return 0

    async def _create_environment_hooks(self, additional_hooks):
        """Create environment hooks for ROS 2 integration."""
        args = self.context.args
        additional_hooks.extend(
            create_environment_hook(
                "ament_prefix_path",
                Path(args.install_base),
                self.context.pkg.name,
                "AMENT_PREFIX_PATH",
                "",
                mode="prepend",
            )
        )

    def _build_cmd(self, cargo_args):
        """Build the cargo build command.

        Since bindings are generated at workspace-level, we just need to run cargo build.
        The .cargo/config.toml has already been written with all the patches.
        """
        cmd = ["cargo", "build"]

        # Handle None cargo_args
        if cargo_args is None:
            cargo_args = []

        # Add all cargo arguments
        cmd.extend(cargo_args)

        return cmd

    def _install_package(self):
        """Install package binaries and create ament markers.

        Calls 'cargo ros2 install' to handle installation to ament layout.
        """
        args = self.context.args

        # Determine build profile
        profile = "release" if hasattr(args, "release") and args.release else "debug"

        # Build cargo ros2 install command
        cmd = [
            "cargo",
            "ros2",
            "install",
            "--install-base",
            args.install_base,
            "--profile",
            profile,
        ]

        # Execute installation
        try:
            result = subprocess.run(
                cmd, cwd=self.context.pkg.path, check=False, capture_output=True
            )

            if result.returncode != 0:
                logger.error(f"cargo ros2 install failed: {result.stderr.decode()}")
                return result.returncode

            logger.info("Package installed successfully")
            return 0

        except FileNotFoundError:
            logger.error(
                "\n\ncargo-ros2 not found!"
                "\n\nPlease ensure cargo-ros2 is installed:"
                "\n  $ cargo install --path cargo-ros2\n"
            )
            return 1
