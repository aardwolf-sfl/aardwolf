#include "utils.h"

#include <dirent.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <unistd.h>

struct dir_iter_private {
  DIR *dp;
  char *cursor;
};

int make_dir(const char *path) {
  const char *until = path;
  struct stat st;

  size_t length = strlen(path) + 1;
  char *temp = (char *)malloc(length);
  memset(temp, 0, length);

  while ((until = strstr(until, "/"))) {
    strncpy(temp, path, until - path);
    if (stat(temp, &st) == -1) {
      if (mkdir(temp, 0700) == -1) {
        return 1;
      }
    }

    until++;
  }

  strcpy(temp, path);
  if (stat(temp, &st) == -1) {
    if (mkdir(temp, 0700) == -1) {
      return 1;
    }
  }

  return 0;
}

#include <stdio.h>

int clean_dir(const char *path) {
  DIR *dp = opendir(path);
  size_t length = strlen(path) + 1;
  char fullpath[256];
  strcpy(fullpath, path);
  fullpath[length - 1] = '/';

  if (dp != NULL) {
    struct dirent *file;
    while ((file = readdir(dp))) {
      if (strcmp(file->d_name, ".") != 0 && strcmp(file->d_name, "..") != 0) {
        strcpy(fullpath + length, file->d_name);
        remove(fullpath);
      }
    }

    closedir(dp);
  } else {
    return 1;
  }

  return 0;
}

int list_dir(const char *path, struct dir_iter *iter) {
  DIR *dp = opendir(path);

  iter->file = NULL;
  iter->ext = NULL;
  iter->__private = NULL;

  if (dp == NULL) {
    return 1;
  }

  struct dir_iter_private *private =
      (struct dir_iter_private *)malloc(sizeof(struct dir_iter_private));

  iter->file = (char *)malloc(256);
  iter->__private = (void *)private;
  memset(iter->file, 0, 256);

private
  ->dp = dp;
private
  ->cursor = stpcpy(iter->file, path);
private
  ->cursor = stpcpy(private->cursor, "/");
  dir_iter_next(iter);

  return 0;
}

char *find_ext(char *file) {
  size_t length = strlen(file);
  while (length--) {
    if (file[length] == '.') {
      return file + length + 1;
    }
  }

  return NULL;
}

int dir_iter_next(struct dir_iter *iter) {
  struct dir_iter_private *private = (struct dir_iter_private *)iter->__private;
  struct dirent *file;
  if ((file = readdir(private->dp))) {
    if (strcmp(file->d_name, ".") != 0 && strcmp(file->d_name, "..") != 0) {
      strcpy(private->cursor, file->d_name);
      iter->ext = find_ext(file->d_name);
      return 1;
    }

    return dir_iter_next(iter);
  } else {
    free(iter->file);
    iter->file = NULL;
    free(iter->__private);
    return 0;
  }
}

int count_levels(const char *path) {
  int counter = 1; // if path is non-empty, then it is a directory
  const char *c = path;
  while ((*c != '\0')) {
    if (*c == '/') {
      counter++;
    }
    c++;
  }
  return counter;
}

int change_dir(const char *path) { return chdir(path) != 0; }

int execute(const char *command) { return system(command) != 0; }
