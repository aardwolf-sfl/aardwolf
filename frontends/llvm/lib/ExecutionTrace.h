#ifndef AARDWOLF_EXECUTION_TRACE_H
#define AARDWOLF_EXECUTION_TRACE_H

#include "llvm/IR/Function.h"
#include "llvm/IR/IRBuilder.h"
#include "llvm/IR/Type.h"
#include "llvm/Pass.h"

#include "Statement.h"

namespace aardwolf {
class ExecutionTrace : public llvm::FunctionPass {
private:
  llvm::FunctionType *WriteStmtTy;
  llvm::FunctionCallee WriteStmt;

public:
  static char ID;

  ExecutionTrace() : FunctionPass(ID) {}

  virtual bool doInitialization(llvm::Module &M);
  virtual bool runOnFunction(llvm::Function &F);
  virtual void getAnalysisUsage(llvm::AnalysisUsage &AU) const;
};
} // namespace aardwolf

#endif // AARDWOLF_EXECUTION_TRACE_H
