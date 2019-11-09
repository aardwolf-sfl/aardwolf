#ifndef AARDWOLF_STATEMENT_H
#define AARDWOLF_STATEMENT_H

#include <set>

#include "llvm/IR/Value.h"
#include "llvm/IR/DebugInfoMetadata.h"
#include "llvm/IR/Instructions.h"

namespace aardwolf {

enum ValueType { Scalar,
    Struct,
    Pointer };

struct Value {
    ValueType Type;
    const llvm::Value* Base;
    const llvm::Value* Accessor;

    static std::shared_ptr<Value> Scalar(const llvm::Value* Val)
    {
        return std::make_shared<Value>(ValueType::Scalar, Val, nullptr);
    }

    static std::shared_ptr<Value> Struct(const llvm::Value* Instance, const llvm::Value* Field)
    {
        return std::make_shared<Value>(ValueType::Struct, Instance, Field);
    }

    static std::shared_ptr<Value> Pointer(const llvm::Value* Base, const llvm::Value* Offset)
    {
        return std::make_shared<Value>(ValueType::Pointer, Base, Offset);
    }

    Value()
        : Type(ValueType::Scalar)
        , Base(nullptr)
        , Accessor(nullptr)
    {
    }

    Value(ValueType Type, const llvm::Value* Base, const llvm::Value* Accessor)
        : Type(Type)
        , Base(Base)
        , Accessor(Accessor)
    {
    }
};

struct Statement {
    // LLVM instruction itself that represents the statement.
    llvm::Instruction* Instr;

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
};

}

#endif // AARDWOLF_STATEMENT_H
