#ifndef AARDWOLF_DYNAMIC_DATA_H
#define AARDWOLF_DYNAMIC_DATA_H

#include "llvm/IR/Module.h"
#include "llvm/IR/PassManager.h"
#include "llvm/Pass.h"

#include "StatementRepository.h"

namespace aardwolf {

struct DynamicDataBase {
  bool runBase(llvm::Module &M, StatementRepository &Repo);
};

struct DynamicData : public llvm::PassInfoMixin<DynamicData>,
                     public DynamicDataBase {
  llvm::PreservedAnalyses run(llvm::Module &M,
                              llvm::ModuleAnalysisManager &MAM);
};

struct LegacyDynamicData : public llvm::ModulePass, public DynamicDataBase {
  static char ID;
  LegacyDynamicData();

  virtual bool runOnModule(llvm::Module &M);
  virtual void getAnalysisUsage(llvm::AnalysisUsage &AU) const;
};

} // namespace aardwolf

#endif // AARDWOLF_DYNAMIC_DATA_H
