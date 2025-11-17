# Licensed under the Apache License, Version 2.0

from pathlib import Path

from colcon_core.logging import colcon_logger
from colcon_core.package_augmentation import PackageAugmentationExtensionPoint
from colcon_core.plugin_system import satisfies_version

logger = colcon_logger.getChild(__name__)


class RustBindingAugmentation(PackageAugmentationExtensionPoint):
    """Generate workspace-level ROS 2 Rust bindings during package augmentation phase.

    This extension runs AFTER package discovery but BEFORE any build tasks start.
    It receives ALL discovered packages and generates bindings once for the entire workspace.

    This is the architecturally correct way to handle workspace-level operations in colcon,
    avoiding fragile directory scanning and respecting colcon's package selection flags.
    """

    PRIORITY = 90  # Run after most other augmentations

    def __init__(self):
        """Initialize the RustBindingAugmentation extension."""
        super().__init__()
        satisfies_version(PackageAugmentationExtensionPoint.EXTENSION_POINT_VERSION, "^1.0")
        self._bindings_generated = False

    def augment_packages(self, descs, *, additional_argument_names=None):
        """Generate workspace-level ROS 2 Rust bindings for all discovered packages.

        Args:
            descs: Collection of ALL package descriptors discovered by colcon
            additional_argument_names: Additional argument names (unused)
        """
        # Only generate bindings once for the entire workspace
        if self._bindings_generated:
            return

        # Collect all packages that have ROS interfaces
        interface_packages = {}
        for desc in descs:
            pkg_path = Path(desc.path)

            # Check if package has interface definitions
            has_interfaces = any(
                [
                    (pkg_path / "msg").exists(),
                    (pkg_path / "srv").exists(),
                    (pkg_path / "action").exists(),
                ]
            )

            if has_interfaces:
                interface_packages[desc.name] = pkg_path
                logger.debug(f"Found interface package: {desc.name} at {pkg_path}")

        if not interface_packages:
            logger.debug("No interface packages found in workspace")
            return

        logger.info(f"Discovered {len(interface_packages)} interface packages via colcon")

        # Store interface packages in generator for use during build phase
        # The first build task will trigger actual binding generation
        # We can't generate here because we don't have access to args/build_base yet
        # Instead, we'll pass the discovered packages to the build task

        # Store in a class variable that build tasks can access
        RustBindingAugmentation._interface_packages = interface_packages
        self._bindings_generated = True

        # Note: We don't call super().augment_packages() because we're doing
        # workspace-level operations, not per-package augmentation


# Class variable to share discovered packages with build tasks
RustBindingAugmentation._interface_packages = {}
