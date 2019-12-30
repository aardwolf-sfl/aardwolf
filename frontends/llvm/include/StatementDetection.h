#ifndef AARDWOLF_STATEMENT_DETECTION_H
#define AARDWOLF_STATEMENT_DETECTION_H

#include "llvm/IR/Module.h"
#include "llvm/IR/PassManager.h"
#include "llvm/Pass.h"

#include "Statement.h"
#include "StatementRepository.h"

namespace aardwolf {

struct StatementDetection : public llvm::AnalysisInfoMixin<StatementDetection> {
  using Result = StatementRepository;

  Result run(llvm::Module &M, llvm::ModuleAnalysisManager &);

  static llvm::AnalysisKey Key;
};

} // namespace aardwolf

#endif // AARDWOLF_STATEMENT_DETECTION_H
