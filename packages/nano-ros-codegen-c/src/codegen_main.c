/**
 * @file codegen_main.c
 * @brief Thin wrapper around nano_ros_codegen_generate_c().
 *
 * Usage: nano_ros_codegen --args-file <path> [--verbose]
 *
 * This is compiled by CMake at configure time and linked against
 * libnano_ros_codegen_c.a to produce a self-contained code generation tool.
 */

#include "nano_ros_codegen.h"
#include <stdio.h>
#include <string.h>

int main(int argc, char *argv[]) {
    const char *args_file = NULL;
    int verbose = 0;

    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "--args-file") == 0 && i + 1 < argc) {
            args_file = argv[++i];
        } else if (strcmp(argv[i], "--verbose") == 0) {
            verbose = 1;
        }
    }

    if (!args_file) {
        fprintf(stderr, "Usage: %s --args-file <path> [--verbose]\n",
                argc > 0 ? argv[0] : "nano_ros_codegen");
        return 1;
    }

    return nano_ros_codegen_generate_c(args_file, verbose);
}
