#include "DynamicData.h"

#include "llvm/IR/IRBuilder.h"
#include "llvm/IR/Instructions.h"
#include "llvm/IR/Module.h"

#include "Statement.h"
#include "StatementDetection.h"
#include "StatementRepository.h"

using namespace aardwolf;

llvm::PreservedAnalyses DynamicData::run(llvm::Module &M,
                                         llvm::ModuleAnalysisManager &MAM) {
  auto &Ctx = M.getContext();
  auto VoidTy = llvm::Type::getVoidTy(Ctx);
  auto StatementRefTy = llvm::Type::getInt64Ty(Ctx);

  std::vector<llvm::Type *> WriteParams;
  WriteParams.push_back(StatementRefTy);

  auto WriteStmtTy = llvm::FunctionType::get(VoidTy, WriteParams, false);
  auto WriteStmt =
      M.getOrInsertFunction("aardwolf_write_statement", WriteStmtTy);

  auto Repo = MAM.getResult<StatementDetection>(M);
  llvm::IRBuilder<> Builder(Ctx);

  for (auto &F : M) {
    if (F.isDeclaration()) {
      continue;
    }

    for (auto I : Repo.FuncInstrsMap[&F]) {
      std::vector<llvm::Value *> Args;
      Builder.SetInsertPoint(I);
      Args.push_back(llvm::ConstantInt::get(
          StatementRefTy, Repo.getStatementId(Repo.InstrStmtMap[I])));

      Builder.CreateCall(WriteStmt, Args);
    }
  }

  return llvm::PreservedAnalyses::none();
}
