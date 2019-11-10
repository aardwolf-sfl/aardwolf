#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "argtable3/argtable3.h"

#include "utils.h"

struct arg_lit *help;
struct arg_rem *compiler_flags_delim;
struct arg_file *sources, *tests, *output, *compiler_flags;
struct arg_str *test_script, *parse_test_output, *language;
struct arg_end *end;


int error(const char *message) {
  fprintf(stderr, "%s\n", message);
  return 1;
}

char *relative_path(char *dest, int levels) {
  while (levels--) {
    dest = stpcpy(dest, "../");
  }

  return dest;
}

const char *prepare_output_dir() {
  const char aardwolf_dir[] = "aardwolf";
  char *output_dir;

  if (output->count == 1) {
    size_t length = strlen(output->filename[0]) + 1;
    output_dir = (char *)malloc(length + sizeof(aardwolf_dir));

    strcpy(output_dir, output->filename[0]);
    output_dir[length - 1] = '/';
    strcpy(output_dir + length, aardwolf_dir);
  } else {
    output_dir = (char *)malloc(sizeof(aardwolf_dir));
    strcpy(output_dir, aardwolf_dir);
  }

  int exitcode = make_dir(output_dir);

  if (exitcode) {
    free(output_dir);
    return NULL;
  }

  exitcode = clean_dir(output_dir);

  if (exitcode) {
    free(output_dir);
    return NULL;
  }

  return output_dir;
}

int prepare_precompilation(char *command, int base_path_levels) {
  char *cursor = command;
  cursor = stpcpy(cursor, "clang -g -c -emit-llvm ");

  for (int s = 0; s < sources->count; s++) {
    cursor = relative_path(cursor, base_path_levels);
    cursor = stpcpy(cursor, sources->filename[s]);
    cursor = stpcpy(cursor, " ");
  }

  return 0;
}

int prepare_analysis(char *command, char *bitcode_file) {
  char *llvm_path = getenv("AARDWOLF_LLVM_PATH");
  if (llvm_path == NULL) {
    return 1;
  }

  char *instrumented_file = (char *)malloc(strlen(bitcode_file) + 5);
  char *cursor = instrumented_file;
  cursor = stpcpy(cursor, bitcode_file);
  strcpy(cursor - 2, "bin.bc");

  sprintf(command,
          "opt -load %s/libLLVMStatementDetection.so -load "
          "%s/libLLVMStaticData.so -load %s/libLLVMExecutionTrace.so "
          "-aard-static-data -aard-exec-trace %s > %s",
          llvm_path, llvm_path, llvm_path, bitcode_file, instrumented_file);

  free(instrumented_file);

  return 0;
}

int prepare_compilation(char *command, const char *output_dir) {
  char *runtime_path = getenv("AARDWOLF_RUNTIME_PATH");
  if (runtime_path == NULL) {
    return 1;
  }

  char *tests_str = (char *)malloc(0x1000);
  char *cursor = tests_str;
  for (int t = 0; t < tests->count; t++) {
    cursor = stpcpy(cursor, tests->filename[t]);
    cursor = stpcpy(cursor, " ");
  }

  sprintf(command,
          "clang -g -o %s/!run %s %s/*.bin.bc %s/libaardwolf_runtime.a ",
          output_dir, tests_str, output_dir, runtime_path);

  free(tests_str);

  return 0;
}

int prepare_running(char *command, const char *output_dir) {
  sprintf(command, "%s/!run > %s/!aardwolf.test", output_dir, output_dir);
  return 0;
}

int aardwolf_llvm() {
  if (language->count > 0 && strcmp(language->sval[0], "c") != 0) {
    return error("Unsupported programming language!");
  }

  const char *output_dir = prepare_output_dir();

  if (!output_dir) {
    return error("Cannot prepare the output directory! Check if you have valid "
                 "permissions.");
  }

  int output_dir_levels = count_levels(output_dir);

  char *command = (char *)malloc(0x10000);
  char *cursor = command;
  memset(command, 0, 0x10000);

  // Change directory to output directory so that clang generates bitcode files
  // there.
  change_dir(output_dir);

  // Compile bitcode files.
  prepare_precompilation(command, output_dir_levels);
  printf("aardwolf: %s  (in %s)\n", command, output_dir);
  execute(command);

  // Change the directory back to the original.
  relative_path(cursor, output_dir_levels);
  change_dir(command);

  // Set data directory so Aardwolf generates files to proper destination.
  setenv("AARDWOLF_DATA_DEST", output_dir, 1);

  struct dir_iter iter;
  list_dir(output_dir, &iter);
  while (iter.file != NULL) {
    if (iter.ext != NULL && strcmp(iter.ext, "bc") == 0) {
      // Generate static data and do the instrumentation.
      prepare_analysis(command, iter.file);
      printf("aardwolf: %s\n", command);
      execute(command);
    }
    dir_iter_next(&iter);
  }

  // Compile an executable.
  prepare_compilation(command, output_dir);
  printf("aardwolf: %s\n", command);
  execute(command);

  // Run the tests.
  prepare_running(command, output_dir);
  printf("aardwolf: %s\n", command);
  execute(command);

  free(command);

  return 0;
}

int main(int argc, char *argv[]) {
  // TODO: Use configuration files rather than CLI options.
  void *argtable[] = {
      help = arg_lit0("h", "help", "displays this help and exits"),
      language = arg_str0(
          "l", "lang", "<c>",
          "programming language of source code [supported: c], default: c"),
      sources =
          arg_filen("s", "sources", NULL, 1, 5,
                    "directiories and/or files that contain application code"),
      tests = arg_filen("t", "tests", NULL, 0, 5,
                        "directiories and/or files that contain testing code "
                        "(might be omitted if --test-script is provided)"),
      output = arg_file0("o", "output", "PATH",
                         "temporary directory for Aardwolf data"),
      test_script =
          arg_str0(NULL, "test-script", "<command>", "custom test command"),
      parse_test_output = arg_str0(
          NULL, "parse-test-output", "<command>",
          "output of test run is expected to be int Aardwolf-compatible "
          "format, this command serves as a converter if it is not such case"),
      compiler_flags_delim = arg_rem("--", NULL),
      compiler_flags = arg_filen(
          NULL, NULL, "compiler flags", 0, argc + 2,
          "command-line flags that are passed to the underlying compiler to "
          "specify compilation details"),
      end = arg_end(3),
  };

  int exitcode = 0;
  char progname[] = "aardwolf_llvm";

  int nerrors;
  nerrors = arg_parse(argc, argv, argtable);

  if (help->count > 0) {
    printf("Usage: %s", progname);
    arg_print_syntax(stdout, argtable, "\n");
    printf("Use Aardwolf fault localization tool with LLVM.\n\n");
    arg_print_glossary(stdout, argtable, "  %-30s %s\n");
    exitcode = 0;
  } else if (nerrors > 0) {
    arg_print_errors(stdout, end, progname);
    printf("Try '%s --help' for more information.\n", progname);
    exitcode = 1;
  } else {
    exitcode = aardwolf_llvm();
  }

  arg_freetable(argtable, sizeof(argtable) / sizeof(argtable[0]));
  return exitcode;
}