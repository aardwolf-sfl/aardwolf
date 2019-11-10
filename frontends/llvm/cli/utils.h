#ifndef AARDWOLF_LLVM_UTILS_H
#define AARDWOLF_LLVM_UTILS_H

struct dir_iter {
  char *file;
  char *ext;
  void *__private;
};

// Creates the directory. Does not give an error if it already exists. Makes
// parents as needed.
int make_dir(const char *path);

// Deletes all files in given directory.
int clean_dir(const char *path);

int list_dir(const char *path, struct dir_iter *iter);

int dir_iter_next(struct dir_iter *iter);

int count_levels(const char *path);

int change_dir(const char *path);

int execute(const char *command);

#endif // AARDWOLF_LLVM_UTILS_H
