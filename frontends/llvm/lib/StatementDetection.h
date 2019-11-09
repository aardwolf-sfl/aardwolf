#ifndef AARDWOLF_STATEMENT_DETECTION_H
#define AARDWOLF_STATEMENT_DETECTION_H

#include <map>

#include "llvm/IR/Instruction.h"
#include "llvm/IR/Value.h"
#include "llvm/Pass.h"

#include "Statement.h"
#include "StatementRepository.h"

namespace aardwolf {
class StatementDetection : public llvm::ModulePass {
private:
  Statement runOnInstruction(llvm::Instruction *I) const;

public:
  static char ID;
  StatementRepository Repo;

  StatementDetection() : ModulePass(ID) {}

  virtual bool runOnModule(llvm::Module &M);
  virtual void getAnalysisUsage(llvm::AnalysisUsage &AU) const;
};
} // namespace aardwolf

#endif // AARDWOLF_STATEMENT_DETECTION_H
