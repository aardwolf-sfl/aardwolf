#include "ExecutionTrace.h"

#include "llvm/IR/IRBuilder.h"
#include "llvm/IR/Instructions.h"
#include "llvm/IR/Module.h"

#include "StatementDetection.h"

using namespace aardwolf;

bool ExecutionTrace::doInitialization(llvm::Module &M) {
  auto VoidTy = llvm::Type::getVoidTy(M.getContext());
  auto StatementRefTy = llvm::Type::getInt64Ty(M.getContext());

  std::vector<llvm::Type *> Params;
  Params.push_back(StatementRefTy); // Statement

  WriteStmtTy = llvm::FunctionType::get(VoidTy, Params, false);
  WriteStmt = M.getOrInsertFunction("aardwolf_write_statement", WriteStmtTy);

  return true;
}

bool ExecutionTrace::runOnFunction(llvm::Function &F) {
  llvm::IRBuilder<> Builder(F.getContext());
  auto StatementRefTy = llvm::Type::getInt64Ty(F.getParent()->getContext());

  auto Repo = getAnalysis<StatementDetection>().Repo;
  for (auto I : Repo.FuncMap[&F]) {
    std::vector<llvm::Value *> Args;
    Builder.SetInsertPoint(I);
    Args.push_back(llvm::ConstantInt::get(
        StatementRefTy, Repo.getStatementId(Repo.StmtMap[I])));

    Builder.CreateCall(WriteStmt, Args);
  }

  return true;
}

void ExecutionTrace::getAnalysisUsage(llvm::AnalysisUsage &AU) const {
  AU.setPreservesCFG();
  AU.addRequired<StatementDetection>();
}

char ExecutionTrace::ID = 0;
static llvm::RegisterPass<ExecutionTrace> X("aard-exec-trace",
                                            "Aardwolf Execution Trace Pass");
