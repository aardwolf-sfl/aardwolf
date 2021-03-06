#ifndef AARDWOLF_STATEMENT_H
#define AARDWOLF_STATEMENT_H

#include <unordered_set>

#include "llvm/ADT/SmallSet.h"

#include "llvm/IR/DebugInfoMetadata.h"
#include "llvm/IR/Instructions.h"
#include "llvm/IR/Value.h"

namespace aardwolf {

enum AccessType { Structural, ArrayLike };

struct Access {
private:
  const llvm::Value *Value;

  const std::shared_ptr<Access> Base;
  const std::vector<Access> Accessors;
  const AccessType Type;

  Access(const llvm::Value *Value)
      : Value(Value), Base(nullptr), Type(AccessType::Structural) {}

  Access(const Access Base, const std::vector<Access> Accessors,
         const AccessType Type)
      : Value(nullptr), Base(std::make_shared<Access>(Base)),
        Accessors(Accessors.begin(), Accessors.end()), Type(Type) {}

public:
  static Access makeScalar(const llvm::Value *Value);
  static Access makeStructural(Access Base, Access Field);
  static Access makeArrayLike(Access Base, Access Index);
  static Access makeArrayLike(Access Base, std::vector<Access> Index);

  bool isScalar() const;

  const llvm::Value *getValue() const;

  const Access &getBase() const;
  const std::vector<Access> &getAccessors() const;
  const AccessType &getType() const;

  const llvm::Value *getValueOrBase() const;

  void print(llvm::raw_ostream &Stream) const;

  std::size_t hash() const {
    if (isScalar()) {
      return std::hash<const llvm::Value *>()(Value);
    } else {
      auto h1 = std::hash<const AccessType>()(Type);
      // auto h2 = std::hash<const Access *>()(Base.get());
      auto h2 = Base->hash();

      std::size_t h = h1 ^ (h2 << 1);

      for (std::vector<Access>::size_type i = 0; i < Accessors.size(); i++) {
        h = h ^ (Accessors[i].hash() << (i + 2));
      }

      return h;
    }
  }

  friend bool operator==(const Access &lhs, const Access &rhs) {
    if (lhs.isScalar() && rhs.isScalar()) {
      return lhs.Value == rhs.Value;
    } else if (!lhs.isScalar() && !rhs.isScalar()) {
      return lhs.Type == rhs.Type && *lhs.Base == *rhs.Base;
    } else {
      return false;
    }
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
  bool isCall() const;
};

} // namespace aardwolf

#endif // AARDWOLF_STATEMENT_H
