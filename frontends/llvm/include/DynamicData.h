#ifndef AARDWOLF_DYNAMIC_DATA_H
#define AARDWOLF_DYNAMIC_DATA_H

#include "llvm/IR/Module.h"
#include "llvm/IR/PassManager.h"
#include "llvm/Pass.h"

namespace aardwolf {

struct DynamicData : public llvm::PassInfoMixin<DynamicData> {
  llvm::PreservedAnalyses run(llvm::Module &M, llvm::ModuleAnalysisManager &MAM);
};

} // namespace aardwolf

#endif // AARDWOLF_DYNAMIC_DATA_H
