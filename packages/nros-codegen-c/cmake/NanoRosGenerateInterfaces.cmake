#[=======================================================================[.rst:
NanoRosGenerateInterfaces
-------------------------

Generate C or C++ bindings for ROS 2 interface files (.msg, .srv, .action).

This module provides two functions:

``nros_find_interfaces()``
  High-level: reads ``package.xml``, resolves transitive interface
  dependencies via AMENT index (with bundled fallback), and generates
  bindings for all required packages.

  .. code-block:: cmake

    nros_find_interfaces([LANGUAGE C|CPP] [SKIP_INSTALL])

``nros_generate_interfaces()``
  Low-level: generates bindings for a single package.

  .. code-block:: cmake

    nros_generate_interfaces(<target>
      [<interface_files>...]
      [LANGUAGE C|CPP]
      [DEPENDENCIES <packages>...]
      [SKIP_INSTALL]
    )

Prerequisites:
  Run ``just install-local`` (or ``cmake --build && cmake --install``)
  before configuring CMake.

#]=======================================================================]

# Allow callers to override _NANO_ROS_PREFIX (e.g. for in-tree cross-compilation
# where the codegen cmake lives under packages/ but the prefix is the project root).
if(NOT DEFINED _NANO_ROS_PREFIX)
    get_filename_component(_NANO_ROS_PREFIX "${CMAKE_CURRENT_LIST_DIR}/../../.." ABSOLUTE)
endif()
set(_NANO_ROS_CMAKE_DIR "${CMAKE_CURRENT_LIST_DIR}")

# =========================================================================
# Locate the nros-codegen tool
# =========================================================================

if(NOT DEFINED CACHE{_NANO_ROS_CODEGEN_TOOL})
  find_program(_NANO_ROS_CODEGEN_TOOL nros-codegen
    PATHS "${_NANO_ROS_PREFIX}/bin"
    NO_DEFAULT_PATH
  )

  if(NOT _NANO_ROS_CODEGEN_TOOL)
    message(FATAL_ERROR
      "nros-codegen not found in ${_NANO_ROS_PREFIX}/bin\n"
      "Install with:\n"
      "  cmake -S <nros-src> -B build && cmake --build build\n"
      "  cmake --install build --prefix <path>"
    )
  endif()

  set(_NANO_ROS_CODEGEN_TOOL "${_NANO_ROS_CODEGEN_TOOL}"
    CACHE INTERNAL "Path to nros C codegen tool")

  message(STATUS "Found nros codegen tool: ${_NANO_ROS_CODEGEN_TOOL}")
endif()

# =========================================================================
# _nros_resolve_interface(<target> <relpath> <out_var>)
# =========================================================================
function(_nros_resolve_interface target relpath out_var)
  set(${out_var} "NOTFOUND" PARENT_SCOPE)

  # 1. Local file
  set(_local "${CMAKE_CURRENT_SOURCE_DIR}/${relpath}")
  if(EXISTS "${_local}")
    set(${out_var} "${_local}" PARENT_SCOPE)
    return()
  endif()

  # 2. Ament index
  if(DEFINED ENV{AMENT_PREFIX_PATH})
    string(REPLACE ":" ";" _ament_paths "$ENV{AMENT_PREFIX_PATH}")
    foreach(_prefix ${_ament_paths})
      set(_candidate "${_prefix}/share/${target}/${relpath}")
      if(EXISTS "${_candidate}")
        set(${out_var} "${_candidate}" PARENT_SCOPE)
        return()
      endif()
    endforeach()
  endif()

  # 3. Bundled interfaces
  set(_candidate "${_NANO_ROS_PREFIX}/share/nano-ros/interfaces/${target}/${relpath}")
  if(EXISTS "${_candidate}")
    set(${out_var} "${_candidate}" PARENT_SCOPE)
    return()
  endif()
endfunction()

# =========================================================================
# nros_generate_interfaces(<target> <files>...
#     [DEPENDENCIES <deps>...] [SKIP_INSTALL])
# =========================================================================
function(nros_generate_interfaces target)
  cmake_parse_arguments(_ARG
    "SKIP_INSTALL"
    "ROS_EDITION;LANGUAGE"
    "DEPENDENCIES"
    ${ARGN}
  )

  if(NOT DEFINED _ARG_ROS_EDITION OR _ARG_ROS_EDITION STREQUAL "")
    set(_ARG_ROS_EDITION "humble")
  endif()

  if(NOT DEFINED _ARG_LANGUAGE OR _ARG_LANGUAGE STREQUAL "")
    set(_ARG_LANGUAGE "C")
  endif()
  string(TOUPPER "${_ARG_LANGUAGE}" _ARG_LANGUAGE)

  # Resolve or auto-discover interface files
  set(_interface_files "")

  if(_ARG_UNPARSED_ARGUMENTS)
    # Explicit files: resolve each via local + ament + bundled
    foreach(_relpath ${_ARG_UNPARSED_ARGUMENTS})
      _nros_resolve_interface("${target}" "${_relpath}" _abs_path)
      if(_abs_path STREQUAL "NOTFOUND")
        message(FATAL_ERROR
          "nros_generate_interfaces(): cannot find '${_relpath}' for "
          "package '${target}'.\n"
          "  Searched:\n"
          "    ${CMAKE_CURRENT_SOURCE_DIR}/${_relpath}\n"
          "    AMENT_PREFIX_PATH/share/${target}/${_relpath}\n"
          "    ${_NANO_ROS_PREFIX}/share/nano-ros/interfaces/${target}/${_relpath}\n"
          "  Hint: run 'just install-local', or check the file path.")
      endif()
      list(APPEND _interface_files "${_abs_path}")
    endforeach()
  else()
    # Auto-discover: no files specified — search local dirs, ament, bundled
    # 1. Local directories
    file(GLOB _local_msg "${CMAKE_CURRENT_SOURCE_DIR}/msg/*.msg")
    file(GLOB _local_srv "${CMAKE_CURRENT_SOURCE_DIR}/srv/*.srv")
    file(GLOB _local_action "${CMAKE_CURRENT_SOURCE_DIR}/action/*.action")
    list(APPEND _interface_files ${_local_msg} ${_local_srv} ${_local_action})

    # 2. Ament index
    if(NOT _interface_files AND DEFINED ENV{AMENT_PREFIX_PATH})
      string(REPLACE ":" ";" _ament_paths "$ENV{AMENT_PREFIX_PATH}")
      foreach(_prefix ${_ament_paths})
        file(GLOB _ament_msg "${_prefix}/share/${target}/msg/*.msg")
        file(GLOB _ament_srv "${_prefix}/share/${target}/srv/*.srv")
        file(GLOB _ament_action "${_prefix}/share/${target}/action/*.action")
        list(APPEND _interface_files ${_ament_msg} ${_ament_srv} ${_ament_action})
      endforeach()
    endif()

    # 3. Bundled interfaces
    if(NOT _interface_files)
      file(GLOB _bundled_msg "${_NANO_ROS_PREFIX}/share/nano-ros/interfaces/${target}/msg/*.msg")
      file(GLOB _bundled_srv "${_NANO_ROS_PREFIX}/share/nano-ros/interfaces/${target}/srv/*.srv")
      file(GLOB _bundled_action "${_NANO_ROS_PREFIX}/share/nano-ros/interfaces/${target}/action/*.action")
      list(APPEND _interface_files ${_bundled_msg} ${_bundled_srv} ${_bundled_action})
    endif()

    if(NOT _interface_files)
      message(FATAL_ERROR
        "nros_generate_interfaces(): no interface files found for '${target}'.\n"
        "  Searched:\n"
        "    ${CMAKE_CURRENT_SOURCE_DIR}/{msg,srv,action}/\n"
        "    AMENT_PREFIX_PATH/share/${target}/{msg,srv,action}/\n"
        "    ${_NANO_ROS_PREFIX}/share/nano-ros/interfaces/${target}/{msg,srv,action}/\n"
        "  Hint: add msg/*.msg locally, source ROS 2 setup.bash, or run 'just install-local'.")
    endif()
  endif()

  # Output directory — language-specific subdirectory
  if(_ARG_LANGUAGE STREQUAL "CPP")
    set(_subdir "nano_ros_cpp")
    set(_lang_flag "cpp")
  else()
    set(_subdir "nano_ros_c")
    set(_lang_flag "c")
  endif()

  set(_output_dir "${CMAKE_CURRENT_BINARY_DIR}/${_subdir}/${target}")
  file(MAKE_DIRECTORY "${_output_dir}")
  file(MAKE_DIRECTORY "${_output_dir}/msg")
  file(MAKE_DIRECTORY "${_output_dir}/srv")
  file(MAKE_DIRECTORY "${_output_dir}/action")

  # ---- Build JSON arguments file ----
  set(_args_file "${CMAKE_CURRENT_BINARY_DIR}/nano_ros_generate_${_lang_flag}_args__${target}.json")

  set(_files_json "")
  set(_first TRUE)
  foreach(_file ${_interface_files})
    if(NOT _first)
      string(APPEND _files_json ",")
    endif()
    set(_first FALSE)
    string(APPEND _files_json "\n    \"${_file}\"")
  endforeach()

  set(_deps_json "")
  set(_first TRUE)
  foreach(_dep ${_ARG_DEPENDENCIES})
    if(NOT _first)
      string(APPEND _deps_json ",")
    endif()
    set(_first FALSE)
    string(APPEND _deps_json "\n    \"${_dep}\"")
  endforeach()

  file(WRITE "${_args_file}" "{
  \"package_name\": \"${target}\",
  \"output_dir\": \"${_output_dir}\",
  \"interface_files\": [${_files_json}
  ],
  \"dependencies\": [${_deps_json}
  ],
  \"ros_edition\": \"${_ARG_ROS_EDITION}\"
}
")

  # ---- Predict output files ----
  set(_generated_headers "")
  set(_generated_sources "")
  set(_generated_rs_files "")
  foreach(_file ${_interface_files})
    get_filename_component(_name "${_file}" NAME_WE)
    get_filename_component(_ext  "${_file}" EXT)

    # CamelCase → snake_case
    string(REGEX REPLACE "([a-z])([A-Z])" "\\1_\\2" _name_snake "${_name}")
    string(TOLOWER "${_name_snake}" _name_lower)

    # Package name → C identifier (replace - with _)
    string(REPLACE "-" "_" _c_pkg "${target}")

    if(_ext STREQUAL ".msg")
      set(_kind "msg")
    elseif(_ext STREQUAL ".srv")
      set(_kind "srv")
    elseif(_ext STREQUAL ".action")
      set(_kind "action")
    else()
      message(FATAL_ERROR "Unknown interface file extension: ${_ext}")
    endif()

    if(_ARG_LANGUAGE STREQUAL "CPP")
      # C++ generates .hpp headers + .rs FFI glue
      list(APPEND _generated_headers
        "${_output_dir}/${_kind}/${_c_pkg}_${_kind}_${_name_lower}.hpp")
      if(_kind STREQUAL "msg")
        list(APPEND _generated_rs_files
          "${_output_dir}/${_kind}/${_c_pkg}_${_kind}_${_name_lower}_ffi.rs")
      elseif(_kind STREQUAL "srv")
        list(APPEND _generated_rs_files
          "${_output_dir}/${_kind}/${_c_pkg}_${_kind}_${_name_lower}_request_ffi.rs"
          "${_output_dir}/${_kind}/${_c_pkg}_${_kind}_${_name_lower}_response_ffi.rs")
      elseif(_kind STREQUAL "action")
        list(APPEND _generated_rs_files
          "${_output_dir}/${_kind}/${_c_pkg}_${_kind}_${_name_lower}_goal_ffi.rs"
          "${_output_dir}/${_kind}/${_c_pkg}_${_kind}_${_name_lower}_result_ffi.rs"
          "${_output_dir}/${_kind}/${_c_pkg}_${_kind}_${_name_lower}_feedback_ffi.rs")
      endif()
    else()
      # C generates .h headers + .c sources
      list(APPEND _generated_headers
        "${_output_dir}/${_kind}/${_c_pkg}_${_kind}_${_name_lower}.h")
      list(APPEND _generated_sources
        "${_output_dir}/${_kind}/${_c_pkg}_${_kind}_${_name_lower}.c")
    endif()
  endforeach()

  # Umbrella header + optional mod.rs
  if(_ARG_LANGUAGE STREQUAL "CPP")
    list(APPEND _generated_headers "${_output_dir}/${target}.hpp")
    list(APPEND _generated_rs_files "${_output_dir}/mod.rs")
  else()
    list(APPEND _generated_headers "${_output_dir}/${target}.h")
  endif()

  # ---- Custom command ----
  add_custom_command(
    OUTPUT ${_generated_headers} ${_generated_sources} ${_generated_rs_files}
    COMMAND "${_NANO_ROS_CODEGEN_TOOL}" --language "${_lang_flag}" --args-file "${_args_file}"
    DEPENDS ${_interface_files} "${_args_file}"
    WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
    COMMENT "Generating nros ${_ARG_LANGUAGE} interfaces for ${target}"
    VERBATIM
  )

  # ---- Library target ----
  if(_ARG_LANGUAGE STREQUAL "CPP")
    # C++ target: header-only INTERFACE + Rust FFI staticlib for message bindings
    set(_lib_target "${target}__nano_ros_cpp")
    add_library(${_lib_target} INTERFACE)
    target_include_directories(${_lib_target}
      INTERFACE
        $<BUILD_INTERFACE:${_output_dir}>
        $<BUILD_INTERFACE:${CMAKE_CURRENT_BINARY_DIR}/${_subdir}>
        $<INSTALL_INTERFACE:include/${target}>
    )

    # Custom target to drive codegen (INTERFACE libraries don't trigger custom commands)
    add_custom_target(${_lib_target}_gen DEPENDS ${_generated_headers} ${_generated_rs_files})
    add_dependencies(${_lib_target} ${_lib_target}_gen)

    # ---- Build Rust FFI glue for generated message types ----
    # The generated .rs files provide extern "C" publish/serialize/deserialize
    # functions. We compile them into a static library via cargo.
    if(_generated_rs_files)
      set(_ffi_crate_dir "${CMAKE_CURRENT_BINARY_DIR}/nano_ros_cpp_ffi_${target}")
      set(_ffi_crate_src "${_ffi_crate_dir}/src")
      set(_ffi_target_dir "${_ffi_crate_dir}/target")
      set(_serdes_dir "${_NANO_ROS_PREFIX}/share/nano-ros/rust/nros-serdes")

      # Cross-compilation: when Rust_CARGO_TARGET is set (e.g. by a CMake
      # toolchain file), pass --target to cargo and adjust the output path.
      if(DEFINED Rust_CARGO_TARGET)
        set(_ffi_cargo_target_flag "--target" "${Rust_CARGO_TARGET}")
        set(_ffi_lib "${_ffi_target_dir}/${Rust_CARGO_TARGET}/release/libnano_ros_cpp_ffi_${target}.a")
      else()
        set(_ffi_cargo_target_flag "")
        set(_ffi_lib "${_ffi_target_dir}/release/libnano_ros_cpp_ffi_${target}.a")
      endif()

      file(MAKE_DIRECTORY "${_ffi_crate_src}")

      # Generate Cargo.toml from template
      set(FFI_TARGET "${target}")
      set(SERDES_DIR "${_serdes_dir}")
      configure_file(
        "${_NANO_ROS_CMAKE_DIR}/cpp_ffi_Cargo.toml.in"
        "${_ffi_crate_dir}/Cargo.toml"
        @ONLY
      )

      # Generate lib.rs with include!() for cross-package FFI references.
      # Using include!() instead of mod keeps all types in the same scope,
      # so cross-package type references (e.g. builtin_interfaces_msg_time_t
      # used in std_msgs) resolve correctly.
      set(_lib_rs_content "")
      string(APPEND _lib_rs_content "// Auto-generated — do not edit\n")
      string(APPEND _lib_rs_content "#![no_std]\n")
      string(APPEND _lib_rs_content "#![allow(non_camel_case_types)]\n\n")
      string(APPEND _lib_rs_content "#[panic_handler]\n")
      string(APPEND _lib_rs_content "fn panic(_info: &core::panic::PanicInfo) -> ! {\n")
      string(APPEND _lib_rs_content "    loop {}\n")
      string(APPEND _lib_rs_content "}\n\n")
      string(APPEND _lib_rs_content "use nros_serdes::{CdrWriter, CdrReader, SerError, DeserError};\n\n")
      string(APPEND _lib_rs_content "unsafe extern \"C\" {\n")
      string(APPEND _lib_rs_content "    fn nros_cpp_publish_raw(handle: *mut core::ffi::c_void, data: *const u8, len: usize) -> i32\;\n")
      string(APPEND _lib_rs_content "}\n\n")

      # include!() dependency FFI .rs files (so their types are in scope)
      foreach(_dep ${_ARG_DEPENDENCIES})
        if(DEFINED ${_dep}_GENERATED_RS_FILES)
          foreach(_rs_file ${${_dep}_GENERATED_RS_FILES})
            # Skip mod.rs — we use include!() instead
            get_filename_component(_rs_name "${_rs_file}" NAME)
            if(NOT _rs_name STREQUAL "mod.rs")
              string(APPEND _lib_rs_content "include!(\"${_rs_file}\")\;\n")
            endif()
          endforeach()
        endif()
      endforeach()

      # include!() own FFI .rs files
      foreach(_rs_file ${_generated_rs_files})
        get_filename_component(_rs_name "${_rs_file}" NAME)
        if(NOT _rs_name STREQUAL "mod.rs")
          string(APPEND _lib_rs_content "include!(\"${_rs_file}\")\;\n")
        endif()
      endforeach()

      file(WRITE "${_ffi_crate_src}/lib.rs" "${_lib_rs_content}")

      # For Tier 3 targets (e.g. armv7a-nuttx-eabi), generate a .cargo/config.toml
      # with build-std=core and use nightly toolchain.
      set(_ffi_cargo_prefix "")
      if(DEFINED Rust_CARGO_TARGET AND Rust_CARGO_TARGET MATCHES "nuttx")
        file(MAKE_DIRECTORY "${_ffi_crate_dir}/.cargo")
        file(WRITE "${_ffi_crate_dir}/.cargo/config.toml"
          "[build]\ntarget = \"${Rust_CARGO_TARGET}\"\n\n"
          "[unstable]\nbuild-std = [\"core\"]\n\n"
          "[target.${Rust_CARGO_TARGET}]\nlinker = \"arm-none-eabi-gcc\"\n\n"
          "[env]\nCC_armv7a_nuttx_eabi = \"arm-none-eabi-gcc\"\n"
        )
        set(_ffi_cargo_prefix "+nightly")
        # With .cargo/config.toml, --target is set there; don't pass it again
        set(_ffi_cargo_target_flag "")
      endif()

      # Build the FFI staticlib after codegen runs
      add_custom_command(
        OUTPUT "${_ffi_lib}"
        COMMAND cargo ${_ffi_cargo_prefix} build --release --manifest-path "${_ffi_crate_dir}/Cargo.toml"
                --target-dir "${_ffi_target_dir}" ${_ffi_cargo_target_flag}
        DEPENDS ${_generated_rs_files} "${_ffi_crate_dir}/Cargo.toml" "${_ffi_crate_src}/lib.rs"
        WORKING_DIRECTORY "${_ffi_crate_dir}"
        COMMENT "Building Rust FFI glue for ${target} C++ bindings"
        VERBATIM
      )

      add_custom_target(${_lib_target}_ffi DEPENDS "${_ffi_lib}")
      add_dependencies(${_lib_target}_ffi ${_lib_target}_gen)
      # Ensure dependency codegen targets run before our FFI build
      foreach(_dep ${_ARG_DEPENDENCIES})
        if(TARGET ${_dep}__nano_ros_cpp_gen)
          add_dependencies(${_lib_target}_ffi ${_dep}__nano_ros_cpp_gen)
        endif()
      endforeach()
      add_dependencies(${_lib_target} ${_lib_target}_ffi)

      # Import the built staticlib
      add_library(${_lib_target}_ffi_lib STATIC IMPORTED)
      set_target_properties(${_lib_target}_ffi_lib PROPERTIES
        IMPORTED_LOCATION "${_ffi_lib}"
      )
      target_link_libraries(${_lib_target} INTERFACE ${_lib_target}_ffi_lib)
    endif()

    # Link to nros C++ library (prefer installed target, fall back to build-time Corrosion target)
    if(TARGET NanoRos::NanoRosCpp)
      target_link_libraries(${_lib_target} INTERFACE NanoRos::NanoRosCpp)
    elseif(TARGET nros_cpp::nros_cpp)
      target_link_libraries(${_lib_target} INTERFACE nros_cpp::nros_cpp)
    endif()

    # Link dependency libraries
    foreach(_dep ${_ARG_DEPENDENCIES})
      if(TARGET ${_dep}__nano_ros_cpp)
        target_link_libraries(${_lib_target} INTERFACE ${_dep}__nano_ros_cpp)
      endif()
    endforeach()
  else()
    # C target with .c sources
    set(_lib_target "${target}__nano_ros_c")

    if(_generated_sources)
      add_library(${_lib_target} STATIC ${_generated_sources})
      target_include_directories(${_lib_target}
        PUBLIC
          $<BUILD_INTERFACE:${_output_dir}>
          $<BUILD_INTERFACE:${CMAKE_CURRENT_BINARY_DIR}/${_subdir}>
          $<INSTALL_INTERFACE:include/${target}>
      )
    else()
      add_library(${_lib_target} INTERFACE)
      target_include_directories(${_lib_target}
        INTERFACE
          $<BUILD_INTERFACE:${_output_dir}>
          $<BUILD_INTERFACE:${CMAKE_CURRENT_BINARY_DIR}/${_subdir}>
          $<INSTALL_INTERFACE:include/${target}>
      )
    endif()

    # Link to nros-c
    if(TARGET NanoRos::NanoRos)
      set(_link_type PUBLIC)
      if(NOT _generated_sources)
        set(_link_type INTERFACE)
      endif()
      target_link_libraries(${_lib_target} ${_link_type} NanoRos::NanoRos)
    elseif(TARGET nros_c::nros_c)
      set(_link_type PUBLIC)
      if(NOT _generated_sources)
        set(_link_type INTERFACE)
      endif()
      target_link_libraries(${_lib_target} ${_link_type} nros_c::nros_c)
    endif()

    # Link dependency libraries
    foreach(_dep ${_ARG_DEPENDENCIES})
      if(TARGET ${_dep}__nano_ros_c)
        set(_link_type PUBLIC)
        if(NOT _generated_sources)
          set(_link_type INTERFACE)
        endif()
        target_link_libraries(${_lib_target} ${_link_type} ${_dep}__nano_ros_c)
      endif()
    endforeach()
  endif()

  # Install
  if(NOT _ARG_SKIP_INSTALL)
    if(_ARG_LANGUAGE STREQUAL "CPP")
      install(
        DIRECTORY "${_output_dir}/"
        DESTINATION "include/${target}"
        FILES_MATCHING PATTERN "*.hpp"
      )
    else()
      install(
        DIRECTORY "${_output_dir}/"
        DESTINATION "include/${target}"
        FILES_MATCHING PATTERN "*.h"
      )
      if(_generated_sources)
        install(TARGETS ${_lib_target}
          EXPORT ${target}Targets
          ARCHIVE DESTINATION lib
          LIBRARY DESTINATION lib
        )
      endif()
    endif()
    install(EXPORT ${target}Targets
      FILE ${target}Targets.cmake
      NAMESPACE ${target}::
      DESTINATION "lib/cmake/${target}"
    )
  endif()

  # Export variables for downstream
  set(${target}_INCLUDE_DIRS "${_output_dir}" PARENT_SCOPE)
  set(${target}_LIBRARIES "${_lib_target}" PARENT_SCOPE)
  set(${target}_GENERATED_HEADERS "${_generated_headers}" PARENT_SCOPE)
  set(${target}_GENERATED_SOURCES "${_generated_sources}" PARENT_SCOPE)
  set(${target}_GENERATED_RS_FILES "${_generated_rs_files}" PARENT_SCOPE)
endfunction()


# =========================================================================
# nros_find_interfaces()
#
# High-level function that reads package.xml from the current source
# directory, resolves transitive interface dependencies via AMENT index
# (with bundled fallback), and generates bindings for all required
# packages in topological order.
#
# Usage:
#   nros_find_interfaces([LANGUAGE CPP] [SKIP_INSTALL])
# =========================================================================
function(nros_find_interfaces)
  cmake_parse_arguments(_ARG
    "SKIP_INSTALL"
    "PACKAGE_XML;LANGUAGE;ROS_EDITION"
    ""
    ${ARGN}
  )

  if(NOT DEFINED _ARG_PACKAGE_XML OR _ARG_PACKAGE_XML STREQUAL "")
    set(_ARG_PACKAGE_XML "${CMAKE_CURRENT_SOURCE_DIR}/package.xml")
  endif()

  if(NOT EXISTS "${_ARG_PACKAGE_XML}")
    message(FATAL_ERROR
      "nros_find_interfaces: package.xml not found at ${_ARG_PACKAGE_XML}")
  endif()

  if(NOT DEFINED _ARG_LANGUAGE OR _ARG_LANGUAGE STREQUAL "")
    set(_ARG_LANGUAGE "CPP")
  endif()

  if(NOT DEFINED _ARG_ROS_EDITION OR _ARG_ROS_EDITION STREQUAL "")
    set(_ARG_ROS_EDITION "humble")
  endif()

  # 1. Call resolve-deps at configure time
  set(_resolve_output "${CMAKE_CURRENT_BINARY_DIR}/_nros_resolved_deps.cmake")
  execute_process(
    COMMAND "${_NANO_ROS_CODEGEN_TOOL}" resolve-deps
            --package-xml "${_ARG_PACKAGE_XML}"
            --output-cmake "${_resolve_output}"
    RESULT_VARIABLE _result
    ERROR_VARIABLE _stderr
  )
  if(NOT _result EQUAL 0)
    message(FATAL_ERROR
      "nros-codegen resolve-deps failed (exit ${_result}):\n${_stderr}")
  endif()

  # 2. Include generated cmake with package lists
  include("${_resolve_output}")

  if(NOT _NROS_RESOLVED_PACKAGES)
    message(WARNING "nros_find_interfaces: no interface packages resolved from ${_ARG_PACKAGE_XML}")
    return()
  endif()

  # 3. Generate interfaces for each resolved package in topo order
  foreach(_pkg ${_NROS_RESOLVED_PACKAGES})
    set(_skip "")
    if(_ARG_SKIP_INSTALL)
      set(_skip "SKIP_INSTALL")
    endif()

    nros_generate_interfaces(${_pkg}
      ${_NROS_RESOLVED_${_pkg}_FILES}
      DEPENDENCIES ${_NROS_RESOLVED_${_pkg}_DEPS}
      LANGUAGE ${_ARG_LANGUAGE}
      ROS_EDITION ${_ARG_ROS_EDITION}
      ${_skip}
    )

    # Re-export variables from nros_generate_interfaces to caller's scope
    set(${_pkg}_INCLUDE_DIRS "${${_pkg}_INCLUDE_DIRS}" PARENT_SCOPE)
    set(${_pkg}_LIBRARIES "${${_pkg}_LIBRARIES}" PARENT_SCOPE)
    set(${_pkg}_GENERATED_HEADERS "${${_pkg}_GENERATED_HEADERS}" PARENT_SCOPE)
    set(${_pkg}_GENERATED_SOURCES "${${_pkg}_GENERATED_SOURCES}" PARENT_SCOPE)
    set(${_pkg}_GENERATED_RS_FILES "${${_pkg}_GENERATED_RS_FILES}" PARENT_SCOPE)
  endforeach()
endfunction()
