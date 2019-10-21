#ifndef AARDWOLF_STATEMENT_H
#define AARDWOLF_STATEMENT_H

#include <set>

#include "llvm/IR/Value.h"
#include "llvm/IR/DebugInfoMetadata.h"
#include "llvm/IR/Instructions.h"

namespace aardwolf {

struct Statement {
    // LLVM instruction itself that represents the statement.
    llvm::Instruction* Instr;

    // Set of input values which go into the statement as inputs.
    // These can be variables (either local or global), constants or
    // the results of function calls.
    std::set<const llvm::Value*> In;

    // Value which comes out of the statement as its result.
    // Not all statements have an output value.
    const llvm::Value* Out;

    // This is used for getting the location (line, column and file) for
    // the refering to the statement in the original source code.
    llvm::DebugLoc Loc;

    Statement() : Instr(nullptr), Out(nullptr) {}
};

}

#endif // AARDWOLF_STATEMENT_H
