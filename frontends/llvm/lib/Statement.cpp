#include "Statement.h"

#include <cassert>

using namespace aardwolf;

Access Access::makeScalar(const llvm::Value *Value) { return Access(Value); }

Access Access::makeStructural(Access Base, Access Field) {
  std::vector<Access> Accessors;
  Accessors.push_back(Field);
  return Access(Base, Accessors, AccessType::Structural);
}

Access Access::makeArrayLike(Access Base, Access Index) {
  std::vector<Access> Accessors;
  Accessors.push_back(Index);
  return Access(Base, Accessors, AccessType::ArrayLike);
}

Access Access::makeArrayLike(Access Base, std::vector<Access> Index) {
  return Access(Base, Index, AccessType::ArrayLike);
}

bool Access::isScalar() const { return Value != nullptr; }

const llvm::Value *Access::getValue() const {
  assert(isScalar() && "Access must be scalar to access the value");
  return Value;
}

const Access &Access::getBase() const {
  assert(!isScalar() && "Access must not be scalar to access the base");
  return *Base;
}

const std::vector<Access> &Access::getAccessors() const {
  assert(!isScalar() && "Access must not be scalar to access the accessors");
  return Accessors;
}

const AccessType &Access::getType() const {
  assert(!isScalar() && "Access must not be scalar to access the access type");
  return Type;
}

const llvm::Value *Access::getValueOrBase() const {
  if (isScalar()) {
    return Value;
  } else {
    return Base->getValueOrBase();
  }
}

void Access::print(llvm::raw_ostream &Stream) const {
  if (isScalar()) {
    Stream << "Scalar(";
    Value->print(Stream);
    Stream << ")";
  } else {
    if (Type == AccessType::Structural) {
      Stream << "Structural(";
      Base->print(Stream);
      Stream << " :: ";
      Accessors[0].print(Stream);
      Stream << ")";
    } else if (Type == AccessType::ArrayLike) {
      Stream << "Structural(";
      Base->print(Stream);
      Stream << " :: [";
      for (auto A : Accessors) {
        A.print(Stream);
        Stream << "  ";
      }
      Stream << "])";
    }
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
