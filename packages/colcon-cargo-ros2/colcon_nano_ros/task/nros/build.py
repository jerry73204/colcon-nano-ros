# Licensed under the Apache License, Version 2.0

import json
import os
import shutil
import subprocess
from pathlib import Path

from colcon_core.environment import create_environment_hooks, create_environment_scripts
from colcon_core.logging import colcon_logger
from colcon_core.plugin_system import satisfies_version
from colcon_core.shell import create_environment_hook
from colcon_core.task import TaskExtensionPoint, run

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


# Lock file to ensure workspace-level binding generation runs only once
_bindings_generated = False


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

        # Generate workspace-level bindings (once, first package triggers)
        rc = await self._generate_bindings(pkg, args, lang)
        if rc:
            return rc

        if lang == 'rust':
            return await self._build_rust(pkg, args, platform,
                                          additional_hooks, skip_hook_creation)
        elif lang in ('c', 'cpp'):
            return await self._build_cmake(pkg, args, lang, platform,
                                           additional_hooks, skip_hook_creation)
        else:
            logger.error(f"Unknown language: {lang}")
            return 1

    async def _generate_bindings(self, pkg, args, lang):
        """Generate workspace-level interface bindings (once per workspace).

        Checks NrosBindingAugmentation for collected interface dependencies,
        then runs cargo nano-ros generate-rust/generate-cpp as needed.
        Output goes to build/nros_bindings/<interface_pkg>/.
        """
        global _bindings_generated
        if _bindings_generated:
            return 0

        # Set flag BEFORE any await to prevent concurrent tasks from
        # also entering this path (Python asyncio is cooperative —
        # the flag must be set before yielding control).
        _bindings_generated = True

        from colcon_nano_ros.nros_augmentation import NrosBindingAugmentation

        interface_deps = getattr(NrosBindingAugmentation, '_interface_deps', set())
        if not interface_deps:
            return 0

        # Derive workspace build dir from args.build_base
        # args.build_base is per-package (e.g., build/hello_nros)
        # We want the workspace-level build/ dir
        build_root = Path(args.build_base).resolve().parent
        bindings_dir = build_root / 'nros_bindings'
        bindings_dir.mkdir(parents=True, exist_ok=True)

        NrosBindingAugmentation._bindings_dir = bindings_dir

        logger.info(
            f"Generating nros bindings for: {', '.join(sorted(interface_deps))}"
        )

        # Generate Rust bindings if any Rust nros packages exist
        if NrosBindingAugmentation._needs_rust:
            for dep in sorted(interface_deps):
                dep_dir = bindings_dir / dep
                if dep_dir.exists() and any(dep_dir.rglob('*.rs')):
                    logger.debug(f"Rust bindings for '{dep}' already exist")
                    continue
                cmd = [
                    'cargo', 'nano-ros', 'bindgen',
                    '--package', dep,
                    '--output', str(bindings_dir / dep),
                ]
                rc = await run(self.context, cmd)
                if rc and rc.returncode != 0:
                    logger.error(f"Failed to generate Rust bindings for '{dep}'")
                    return rc.returncode

        # C/C++ bindings are handled by CMake's nano_ros_generate_interfaces()
        # during the cmake build step — not here.

        logger.info(f"nros bindings generated in {bindings_dir}")
        return 0

    async def _build_rust(self, pkg, args, platform,
                          additional_hooks, skip_hook_creation):
        """Build a Rust nros package with cargo."""
        pkg_path = Path(pkg.path)
        install_base = Path(args.install_base)

        # 1. Build with cargo
        cmd = ['cargo', 'build', '--release', '--quiet']

        target = PLATFORM_TARGETS.get(platform)
        if target:
            cmd.extend(['--target', target])

        rc = await run(self.context, cmd, cwd=str(pkg_path))
        if rc and rc.returncode != 0:
            return rc.returncode

        # 2. Find and install binary targets
        binaries = self._find_rust_binaries(pkg_path, target)
        if not binaries:
            logger.warning(f"No binary targets found for '{pkg.name}'")

        lib_dir = install_base / 'lib' / pkg.name
        lib_dir.mkdir(parents=True, exist_ok=True)
        for bin_path in binaries:
            dest = lib_dir / bin_path.name
            shutil.copy2(str(bin_path), str(dest))
            logger.info(f"Installed {bin_path.name} → {dest}")

        # 3. Install package.xml
        share_dir = install_base / 'share' / pkg.name
        share_dir.mkdir(parents=True, exist_ok=True)
        pkg_xml = pkg_path / 'package.xml'
        if pkg_xml.exists():
            shutil.copy2(str(pkg_xml), str(share_dir / 'package.xml'))

        # 4. Create ament resource index marker
        resource_dir = share_dir / 'ament_index' / 'resource_index' / 'packages'
        resource_dir.mkdir(parents=True, exist_ok=True)
        (resource_dir / pkg.name).touch()

        # 5. Create environment hooks
        if not skip_hook_creation:
            hooks = additional_hooks or []
            hooks.extend(
                create_environment_hook(
                    'ament_prefix_path', install_base, pkg.name,
                    'AMENT_PREFIX_PATH', '', mode='prepend',
                )
            )
            default_hooks = create_environment_hooks(
                str(install_base), pkg.name)
            create_environment_scripts(
                pkg, args, default_hooks=default_hooks,
                additional_hooks=hooks)

        return 0

    def _find_rust_binaries(self, pkg_path, target):
        """Find built binary targets using cargo metadata."""
        try:
            result = subprocess.run(
                ['cargo', 'metadata', '--no-deps', '--format-version=1'],
                capture_output=True, text=True, cwd=str(pkg_path))
            if result.returncode != 0:
                logger.warning("cargo metadata failed, scanning target/ dir")
                return self._find_binaries_fallback(pkg_path, target)

            metadata = json.loads(result.stdout)
            bin_names = []
            for package in metadata.get('packages', []):
                for t in package.get('targets', []):
                    if 'bin' in t.get('kind', []):
                        bin_names.append(t['name'])

            if not bin_names:
                return []

            # Resolve binary paths
            if target:
                target_dir = pkg_path / 'target' / target / 'release'
            else:
                target_dir = pkg_path / 'target' / 'release'

            binaries = []
            for name in bin_names:
                bin_path = target_dir / name
                if bin_path.exists():
                    binaries.append(bin_path)
            return binaries

        except Exception as e:
            logger.warning(f"cargo metadata failed: {e}")
            return self._find_binaries_fallback(pkg_path, target)

    async def _build_cmake(self, pkg, args, lang, platform,
                           additional_hooks, skip_hook_creation):
        """Build a C/C++ nros package with CMake."""
        pkg_path = Path(pkg.path).resolve()
        install_base = Path(args.install_base).resolve()
        build_dir = Path(args.build_base).resolve()

        # 1. CMake configure
        cmd = [
            'cmake',
            '-S', str(pkg_path),
            '-B', str(build_dir),
            f'-DCMAKE_INSTALL_PREFIX={install_base}',
        ]

        # Pass CMAKE_PREFIX_PATH so find_package(NanoRos) works.
        # Include both the colcon install prefix and any existing prefix path.
        prefix_paths = [str(install_base.parent)]
        env_prefix = os.environ.get('CMAKE_PREFIX_PATH', '')
        if env_prefix:
            prefix_paths.append(env_prefix)
        cmd.append(f'-DCMAKE_PREFIX_PATH={";".join(prefix_paths)}')

        rc = await run(self.context, cmd)
        if rc and rc.returncode != 0:
            return rc.returncode

        # 2. CMake build
        rc = await run(self.context, ['cmake', '--build', str(build_dir)])
        if rc and rc.returncode != 0:
            return rc.returncode

        # 3. CMake install (uses CMAKE_INSTALL_PREFIX set during configure)
        rc = await run(self.context, ['cmake', '--install', str(build_dir)])
        if rc and rc.returncode != 0:
            return rc.returncode

        # 4. Install package.xml if not already installed by CMake
        share_dir = install_base / 'share' / pkg.name
        share_dir.mkdir(parents=True, exist_ok=True)
        pkg_xml = pkg_path / 'package.xml'
        dest_xml = share_dir / 'package.xml'
        if pkg_xml.exists() and not dest_xml.exists():
            shutil.copy2(str(pkg_xml), str(dest_xml))

        # 5. Create ament resource index marker
        resource_dir = share_dir / 'ament_index' / 'resource_index' / 'packages'
        resource_dir.mkdir(parents=True, exist_ok=True)
        (resource_dir / pkg.name).touch()

        # 6. Create environment hooks
        if not skip_hook_creation:
            hooks = additional_hooks or []
            hooks.extend(
                create_environment_hook(
                    'ament_prefix_path', install_base, pkg.name,
                    'AMENT_PREFIX_PATH', '', mode='prepend',
                )
            )
            default_hooks = create_environment_hooks(
                str(install_base), pkg.name)
            create_environment_scripts(
                pkg, args, default_hooks=default_hooks,
                additional_hooks=hooks)

        return 0

    def _find_binaries_fallback(self, pkg_path, target):
        """Fallback: scan target/release/ for executable files."""
        if target:
            target_dir = pkg_path / 'target' / target / 'release'
        else:
            target_dir = pkg_path / 'target' / 'release'

        if not target_dir.exists():
            return []
        return [
            f for f in target_dir.iterdir()
            if f.is_file() and os.access(str(f), os.X_OK)
            and not f.suffix  # skip .d, .rlib, etc.
        ]
