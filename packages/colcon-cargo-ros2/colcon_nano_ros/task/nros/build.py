# Licensed under the Apache License, Version 2.0

from colcon_core.logging import colcon_logger
from colcon_core.plugin_system import satisfies_version
from colcon_core.task import TaskExtensionPoint

logger = colcon_logger.getChild(__name__)

# Platform → Rust target triple mapping.
# None means native (no --target flag).
PLATFORM_TARGETS = {
    'native': None,
    'freertos': 'thumbv7m-none-eabi',
    'baremetal': 'thumbv7m-none-eabi',
    'nuttx': 'thumbv7m-none-eabi',
    'threadx': 'thumbv7m-none-eabi',  # TODO: also riscv64gc-unknown-none-elf
    'zephyr': None,  # Zephyr uses west build, not cargo --target
}


def parse_nros_type(pkg_type):
    """Parse 'ros.nros.<lang>.<platform>' into (lang, platform).

    >>> parse_nros_type('ros.nros.rust.freertos')
    ('rust', 'freertos')
    >>> parse_nros_type('ros.nros.c.native')
    ('c', 'native')
    """
    parts = pkg_type.split('.')
    if len(parts) != 4 or parts[0] != 'ros' or parts[1] != 'nros':
        raise ValueError(
            f"Invalid nros build type: '{pkg_type}'. "
            f"Expected 'ros.nros.<lang>.<platform>'."
        )
    return parts[2], parts[3]


class NrosBuildTask(TaskExtensionPoint):
    """Build task for nano-ros packages (nros.<lang>.<platform>).

    Supports Rust (cargo), C (cmake), and C++ (cmake) packages
    targeting native, FreeRTOS, bare-metal, NuttX, ThreadX, and Zephyr.

    The build type is encoded in package.xml:
        <build_type>nros.rust.freertos</build_type>

    Board-specific configuration is handled by the board crate (Rust)
    or CMake platform module (C/C++), not by this task.
    """

    def __init__(self):
        super().__init__()
        satisfies_version(TaskExtensionPoint.EXTENSION_POINT_VERSION, '^1.0')

    async def build(self, *, additional_hooks=None, skip_hook_creation=False):
        pkg = self.context.pkg
        args = self.context.args

        lang, platform = parse_nros_type(pkg.type)
        logger.info(
            f"Building nros package '{pkg.name}' "
            f"(lang={lang}, platform={platform})"
        )

        # TODO: 78.2-78.7 — actual build implementation
        logger.warning(
            f"nros build task for '{pkg.name}' is a stub — "
            f"build not yet implemented for {lang}/{platform}"
        )
        return 0
