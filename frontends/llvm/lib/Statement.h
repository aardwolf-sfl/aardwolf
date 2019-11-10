#ifndef AARDWOLF_STATEMENT_H
#define AARDWOLF_STATEMENT_H

#include <set>

#include "llvm/IR/DebugInfoMetadata.h"
#include "llvm/IR/Instructions.h"
#include "llvm/IR/Value.h"

namespace aardwolf {

// Struct and Pointer represent *access* to a structure or pointer.
// That is, it does not covers when whole structs or pointers are assigned to
// variables. In such cases, Scalar is used (as well as in all other cases).
// TODO: More precise naming.
enum ValueType { Scalar, Struct, Pointer };

struct Value {
  ValueType Type;

  // One of the following:
  //   * AllocaInst - represents a local variable.
  //   * CallInst - represents the result of a function call.
  //   * GlobalVariable - represents a global variable.
  //   * GetElementPtrInst - represents a composite variable (struct or
  //   pointer).
  // If the scalar value represents an accessor, this member field can be also:
  //   * Constant - represents a constant.
  const llvm::Value *Base;

  const std::shared_ptr<Value> Accessor;

  static std::shared_ptr<Value> Scalar(const llvm::Value *Val) {
    return std::make_shared<Value>(ValueType::Scalar, Val, nullptr);
  }

  static std::shared_ptr<Value> Struct(const llvm::Value *Instance,
                                       const std::shared_ptr<Value> Field) {
    return std::make_shared<Value>(ValueType::Struct, Instance, Field);
  }

  static std::shared_ptr<Value> Pointer(const llvm::Value *Base,
                                        const std::shared_ptr<Value> Offset) {
    return std::make_shared<Value>(ValueType::Pointer, Base, Offset);
  }

  Value() : Type(ValueType::Scalar), Base(nullptr), Accessor(nullptr) {}

  Value(ValueType Type, const llvm::Value *Base,
        std::shared_ptr<Value> Accessor)
      : Type(Type), Base(Base), Accessor(Accessor) {}
};

struct Statement {
  // LLVM instruction itself that represents the statement.
  llvm::Instruction *Instr;

  // Set of input values which go into the statement as inputs.
  // These can be variables (either local or global), constants or
  // the results of function calls.
  std::set<std::shared_ptr<Value>> In;

  // Value which comes out of the statement as its result.
  // Not all statements have an output value.
  std::shared_ptr<Value> Out;

  // This is used for getting the location (line, column and file) for
  // the refering to the statement in the original source code.
  llvm::DebugLoc Loc;

  Statement() : Instr(nullptr), Out(nullptr) {}

  bool isArg() const {
    // Argument is the first operand of a store instruction (if the instruction
    // represents initialization of local variable with argument value).
    return Instr->getNumOperands() > 0 &&
           llvm::isa<llvm::Argument>(Instr->getOperand(0));
  }

  bool isRet() const { return llvm::isa<llvm::ReturnInst>(Instr); }
};

} // namespace aardwolf

#endif // AARDWOLF_STATEMENT_H
