#ifndef AARDWOLF_STATIC_DATA_H
#define AARDWOLF_STATIC_DATA_H

#include "llvm/IR/Module.h"
#include "llvm/IR/PassManager.h"
#include "llvm/Pass.h"

namespace aardwolf {

struct StaticData : public llvm::PassInfoMixin<StaticData> {
  std::string DestDir;
  StaticData(std::string& DestDir);

  llvm::PreservedAnalyses run(llvm::Module &M, llvm::ModuleAnalysisManager &MAM);
};

} // namespace aardwolf

#endif // AARDWOLF_STATIC_DATA_H
