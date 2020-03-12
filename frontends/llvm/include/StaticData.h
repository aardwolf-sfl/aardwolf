#ifndef AARDWOLF_STATIC_DATA_H
#define AARDWOLF_STATIC_DATA_H

#include "llvm/IR/Module.h"
#include "llvm/IR/PassManager.h"
#include "llvm/IR/LegacyPassManager.h"
#include "llvm/Pass.h"

#include "StatementRepository.h"

namespace aardwolf {

struct StaticDataBase {
  std::string DestDir;
  StaticDataBase();
  StaticDataBase(std::string &DestDir);

  bool runBase(llvm::Module &M, StatementRepository &Repo);
};

struct StaticData : public llvm::PassInfoMixin<StaticData>,
                    public StaticDataBase {
  std::string DestDir;
  StaticData(std::string &DestDir);

  llvm::PreservedAnalyses run(llvm::Module &M,
                              llvm::ModuleAnalysisManager &MAM);
};

struct LegacyStaticData : public llvm::ModulePass, public StaticDataBase {
  static char ID;
  LegacyStaticData();
  LegacyStaticData(std::string &DestDir);

  virtual bool runOnModule(llvm::Module &M);
  virtual void getAnalysisUsage(llvm::AnalysisUsage &AU) const;
};

} // namespace aardwolf

#endif // AARDWOLF_STATIC_DATA_H
