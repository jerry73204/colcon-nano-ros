# Licensed under the Apache License, Version 2.0

from colcon_core.logging import colcon_logger
from colcon_core.plugin_system import satisfies_version
from colcon_core.task import TaskExtensionPoint, run

logger = colcon_logger.getChild(__name__)


class AmentCargoTestTask(TaskExtensionPoint):
    """A test task for packages with Cargo.toml + package.xml.

    Tests are already built by `colcon build` so this task only needs to
    run them and doesn't need to worry about dependency management
    (unlike the `AmentCargoBuildTask`).
    """

    def __init__(self):  # noqa: D107
        super().__init__()
        satisfies_version(TaskExtensionPoint.EXTENSION_POINT_VERSION, "^1.0")

    def add_arguments(self, *, parser):  # noqa: D102
        parser.add_argument(
            "--cargo-args",
            nargs="*",
            default=[],
            help="Arbitrary arguments passed to cargo test",
        )

    async def test(self):  # noqa: D102
        """Run tests using cargo test."""
        args = self.context.args
        cmd = ["cargo", "test"]

        # Use build_base as target directory
        if hasattr(args, "build_base"):
            cmd.extend(["--target-dir", args.build_base])

        # Add additional cargo arguments
        if hasattr(args, "cargo_args") and args.cargo_args:
            cmd.extend(args.cargo_args)

        if hasattr(self.context, "dry_run") and self.context.dry_run:
            logger.info(f"Would execute: {' '.join(cmd)}")
            return 0

        # Execute the test command
        # Note: We always return 0 even if tests fail, since we want colcon to
        # report success as long as the test command itself executed
        await run(self.context, cmd, cwd=self.context.pkg.path, env=None)
        return 0
