# Licensed under the Apache License, Version 2.0

from pathlib import Path
import subprocess

from colcon_core.logging import colcon_logger
from colcon_core.plugin_system import satisfies_version
from colcon_core.shell import create_environment_hook
from colcon_core.task import TaskExtensionPoint, run


logger = colcon_logger.getChild(__name__)


class AmentCargoBuildTask(TaskExtensionPoint):
    """A build task for packages with Cargo.toml + package.xml using cargo-ros2.

    cargo-ros2 handles ROS 2 binding generation, .cargo/config.toml patching,
    and installation automatically. This task is a simple orchestration layer
    that invokes cargo-ros2.

    All dependency resolution and config.toml management is delegated to
    cargo-ros2 to avoid race conditions.
    """

    def __init__(self):  # noqa: D107
        super().__init__()
        satisfies_version(TaskExtensionPoint.EXTENSION_POINT_VERSION, "^1.0")

    def add_arguments(self, *, parser):  # noqa: D102
        parser.add_argument(
            "--lookup-in-workspace",
            action="store_true",
            help="Look up dependencies in the workspace directory. "
            "By default, dependencies are looked up only in the installation "
            "prefixes. This option is useful for setting up a "
            ".cargo/config.toml for subsequent builds with cargo.",
        )

    async def build(self, *, additional_hooks=None):  # noqa: D102
        """Build the Rust ROS 2 package using cargo-ros2."""
        additional_hooks = [] if additional_hooks is None else additional_hooks

        # Prepare: check for cargo-ros2 and create environment hooks
        rc = await self._prepare(additional_hooks)
        if rc:
            return rc

        # Build and install: delegated to cargo-ros2
        args = self.context.args
        cmd = self._build_cmd(args.cargo_args if hasattr(args, "cargo_args") else [])

        # Add --lookup-in-workspace flag if requested
        if args.lookup_in_workspace:
            cmd.append("--lookup-in-workspace")

        # Execute the build command
        result = await run(self.context, cmd, cwd=self.context.pkg.path, env=None)

        # Return the exit code (colcon expects an integer, not CompletedProcess)
        return result.returncode if result else 0

    async def _prepare(self, additional_hooks):
        """Check prerequisites and create environment hooks."""
        # Check for cargo-ros2
        cargo_ros2_check = "cargo ros2 --help".split()
        if subprocess.run(cargo_ros2_check, capture_output=True).returncode != 0:
            logger.error(
                "\n\nament_cargo package found but cargo-ros2 was not detected."
                "\n\nPlease install it by running:"
                "\n $ cargo install cargo-ros2\n"
            )
            return 1

        # Create environment hook for AMENT_PREFIX_PATH
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

        return 0

    def _build_cmd(self, cargo_args):
        """Build the cargo ros2 ament-build command."""
        args = self.context.args
        cmd = [
            "cargo",
            "ros2",
            "ament-build",
            "--install-base",
            args.install_base,
        ]

        # Handle None cargo_args
        if cargo_args is None:
            cargo_args = []

        # Add --release if present in cargo_args
        if "--release" in cargo_args:
            cmd.append("--release")

        # Pass through all additional cargo args
        # (they will be forwarded to cargo build after bindings are generated)
        non_release_args = [arg for arg in cargo_args if arg != "--release"]
        if non_release_args:
            cmd.extend(["--"] + non_release_args)

        return cmd
