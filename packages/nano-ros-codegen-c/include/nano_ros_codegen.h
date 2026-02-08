/**
 * @file nano_ros_codegen.h
 * @brief C interface for nano-ros code generation.
 *
 * This header declares the function provided by libnano_ros_codegen_c.a.
 * The CMake build system uses this to compile a thin wrapper executable
 * that drives C code generation at configure time.
 */

#ifndef NANO_ROS_CODEGEN_H
#define NANO_ROS_CODEGEN_H

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Generate C bindings from a JSON arguments file.
 *
 * @param args_file  Path to the JSON arguments file (null-terminated).
 * @param verbose    Non-zero for verbose output.
 * @return 0 on success, 1 on error (details printed to stderr).
 */
int nano_ros_codegen_generate_c(const char *args_file, int verbose);

#ifdef __cplusplus
}
#endif

#endif  /* NANO_ROS_CODEGEN_H */
