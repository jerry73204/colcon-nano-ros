# packages/codegen/packages/nros-codegen-c/CMakeLists.txt
#
# Builds the nros-codegen binary via Corrosion (Cargo integration).
# The binary is used by NanoRosGenerateInterfaces.cmake to generate
# C bindings from .msg/.srv/.action files.
#
# This file is included from the top-level CMakeLists.txt via add_subdirectory().
# It uses the codegen workspace Cargo.toml (one level up).

corrosion_import_crate(
    MANIFEST_PATH "${CMAKE_CURRENT_SOURCE_DIR}/../Cargo.toml"
    CRATES        nros-codegen-c
    LOCKED
)

# The [[bin]] name in Cargo.toml is "nros-codegen", so Corrosion creates
# a target named "nros-codegen".
install(PROGRAMS "$<TARGET_FILE:nros-codegen>"
    DESTINATION bin
)

# ---- CMake codegen module ----
install(FILES
    "${CMAKE_CURRENT_SOURCE_DIR}/cmake/NanoRosGenerateInterfaces.cmake"
    "${CMAKE_CURRENT_SOURCE_DIR}/cmake/cpp_ffi_Cargo.toml.in"
    "${CMAKE_CURRENT_SOURCE_DIR}/cmake/ffi_lib_rs.in"
    DESTINATION lib/cmake/NanoRos
)
