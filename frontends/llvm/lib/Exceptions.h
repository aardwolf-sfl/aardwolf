#ifndef AARDWOLF_EXCEPTIONS_H
#define AARDWOLF_EXCEPTIONS_H

#include <exception>

struct UnknownLocation : public std::exception {
  virtual const char *what() const throw() {
    return "Could not find the source code location of a statement.";
  }
};

#endif // AARDWOLF_EXCEPTIONS_H
