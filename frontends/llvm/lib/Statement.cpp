#include "Statement.h"

using namespace aardwolf;

Access::Access(const AccessType Type, const llvm::Value *Base)
    : Type(Type), Base(Base) {}

Access::Access(const AccessType Type, const llvm::Value *Base, Access Accessor)
    : Type(Type), Base(Base), Accessors({Accessor}) {}

Access::Access(const AccessType Type, const llvm::Value *Base,
               std::vector<Access> Accessors)
    : Type(Type), Base(Base), Accessors(Accessors.begin(), Accessors.end()) {}

Access Access::makeScalar(const llvm::Value *Value) {
  return Access(AccessType::Scalar, Value);
}

Access Access::makeStructural(const llvm::Value *Base, Access Accessor) {
  return Access(AccessType::Structural, Base, Accessor);
}

Access Access::makeArrayLike(const llvm::Value *Base, Access Offset) {
  return Access(AccessType::ArrayLike, Base, Offset);
}

Access Access::makeArrayLike(const llvm::Value *Base,
                             std::vector<Access> Offset) {
  return Access(AccessType::ArrayLike, Base, Offset);
}

AccessType Access::getType() const { return Type; }

const llvm::Value *Access::getValue() const {
  if (Type == AccessType::Scalar) {
    return Base;
  } else {
    throw "not a scalar access";
  }
}

const llvm::Value *Access::getBase() const {
  if (Type != AccessType::Scalar) {
    return Base;
  } else {
    throw "not a structural or array-like access";
  }
}

const llvm::Value *Access::getValueOrBase() const {
  if (Type == AccessType::Scalar) {
    return getValue();
  } else {
    return getBase();
  }
}

const Access &Access::getAccessor() const {
  if (Type == AccessType::Structural) {
    return Accessors[0];
  } else {
    throw "not a structural access";
  }
}

const std::vector<Access> &Access::getIndexVars() const {
  if (Type == AccessType::ArrayLike) {
    return Accessors;
  } else {
    throw "not an array-like access";
  }
}

LineCol::LineCol(uint32_t Line, uint32_t Col) : Line(Line), Col(Col) {}

Location::Location(std::string File, LineCol Begin, LineCol End)
    : File(File), Begin(Begin), End(End) {}

Statement::Statement()
    : Instr(nullptr), In(16), Out(nullptr),
      Loc("", LineCol(0, 0), LineCol(0, 0)) {}

bool Statement::isArg() const {
  // Argument is the first operand of a store instruction (if the instruction
  // represents initialization of local variable with argument value).
  return Instr->getNumOperands() > 0 &&
         llvm::isa<llvm::Argument>(Instr->getOperand(0));
}

bool Statement::isRet() const { return llvm::isa<llvm::ReturnInst>(Instr); }
