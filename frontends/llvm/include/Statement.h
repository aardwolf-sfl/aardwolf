#ifndef AARDWOLF_STATEMENT_H
#define AARDWOLF_STATEMENT_H

#include <unordered_set>

#include "llvm/ADT/SmallSet.h"

#include "llvm/IR/DebugInfoMetadata.h"
#include "llvm/IR/Instructions.h"
#include "llvm/IR/Value.h"

namespace aardwolf {

enum AccessType { Scalar, Structural, ArrayLike };

struct Access {
private:
  const AccessType Type;

  const llvm::Value *Base;
  const std::vector<Access> Accessors;

  Access(const AccessType Type, const llvm::Value *Base);
  Access(const AccessType Type, const llvm::Value *Base, Access Accessor);
  Access(const AccessType Type, const llvm::Value *Base,
         std::vector<Access> Accessors);

public:
  static Access makeScalar(const llvm::Value *Value);

  static Access makeStructural(const llvm::Value *Base, Access Accessor);

  static Access makeArrayLike(const llvm::Value *Base, Access IndexVars);

  static Access makeArrayLike(const llvm::Value *Base,
                              std::vector<Access> IndexVars);

  AccessType getType() const;
  const llvm::Value *getValue() const;
  const llvm::Value *getBase() const;
  const llvm::Value *getValueOrBase() const;
  const Access &getAccessor() const;
  const std::vector<Access> &getIndexVars() const;

  std::size_t hash() const {
    auto h1 = std::hash<const AccessType>()(Type);
    auto h2 = std::hash<const llvm::Value *>()(Base);

    std::size_t h = h1 ^ (h2 << 1);

    for (std::vector<Access>::size_type i = 0; i < Accessors.size(); i++) {
      h = h ^ (Accessors[i].hash() << (i + 2));
    }

    return h;
  }

  friend bool operator==(const Access &lhs, const Access &rhs) {
    return lhs.Type == rhs.Type && lhs.Base == rhs.Base;
  }

  friend bool operator!=(const Access &lhs, const Access &rhs) {
    return !(lhs == rhs);
  }
};

struct AccessHasher {
  std::size_t operator()(const Access &access) const { return access.hash(); }
};

struct LineCol {
  uint32_t Line;
  uint32_t Col;

  LineCol(uint32_t Line, uint32_t Col);
};

struct Location {
  std::string File;
  LineCol Begin;
  LineCol End;

  Location(std::string File, LineCol Begin, LineCol End);
};

struct Statement {
  // LLVM instruction itself that represents the statement.
  llvm::Instruction *Instr;

  // Set of input values which go into the statement as inputs.
  // These can be variables (either local or global), constants or
  // the results of function calls.
  std::unordered_set<Access, AccessHasher> In;

  // Value which comes out of the statement as its result.
  // Not all statements have an output value.
  std::shared_ptr<Access> Out;

  // Location of the statement in the original source code.
  Location Loc;

  Statement();

  bool isArg() const;
  bool isRet() const;
};

} // namespace aardwolf

#endif // AARDWOLF_STATEMENT_H
