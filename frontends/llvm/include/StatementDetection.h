#ifndef AARDWOLF_STATEMENT_DETECTION_H
#define AARDWOLF_STATEMENT_DETECTION_H

#include "llvm/IR/Module.h"
#include "llvm/IR/PassManager.h"
#include "llvm/Pass.h"

#include "Statement.h"
#include "StatementRepository.h"

namespace aardwolf {

struct StatementDetectionBase {
  StatementRepository Repo;

  bool runBase(llvm::Module &M);
};

struct StatementDetection : public llvm::AnalysisInfoMixin<StatementDetection>,
                            public StatementDetectionBase {
  using Result = StatementRepository;

  Result run(llvm::Module &M, llvm::ModuleAnalysisManager &);

  static llvm::AnalysisKey Key;
};

struct LegacyStatementDetection : public llvm::ModulePass,
                                  public StatementDetectionBase {
  static char ID;

  LegacyStatementDetection();

  virtual bool runOnModule(llvm::Module &M);
  virtual void getAnalysisUsage(llvm::AnalysisUsage &AU) const;
};

} // namespace aardwolf

#endif // AARDWOLF_STATEMENT_DETECTION_H
