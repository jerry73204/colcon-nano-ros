# Licensed under the Apache License, Version 2.0

from colcon_core.logging import colcon_logger
from colcon_core.plugin_system import satisfies_version
from colcon_core.task import TaskExtensionPoint

from .build import parse_nros_type

logger = colcon_logger.getChild(__name__)


class NrosTestTask(TaskExtensionPoint):
    """Test task for nano-ros packages (nros.<lang>.<platform>).

    Runs tests appropriate for the target platform:
    - native: execute binary directly
    - freertos/baremetal: launch QEMU, capture semihosting output
    - zephyr: west flash or native_sim
    """

    def __init__(self):
        super().__init__()
        satisfies_version(TaskExtensionPoint.EXTENSION_POINT_VERSION, '^1.0')

    async def test(self, *, additional_hooks=None):
        pkg = self.context.pkg

        lang, platform = parse_nros_type(pkg.type)
        logger.info(
            f"Testing nros package '{pkg.name}' "
            f"(lang={lang}, platform={platform})"
        )

        # TODO: 78.8 — actual test implementation
        logger.warning(
            f"nros test task for '{pkg.name}' is a stub — "
            f"tests not yet implemented for {lang}/{platform}"
        )
        return 0
