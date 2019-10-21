#ifndef AARDWOLF_STATEMENT_DETECTION_H
#define AARDWOLF_STATEMENT_DETECTION_H

#include <map>

#include "llvm/Pass.h"
#include "llvm/IR/Value.h"
#include "llvm/IR/Instruction.h"

#include "StatementRepository.h"
#include "Statement.h"

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
}

#endif // AARDWOLF_STATEMENT_DETECTION_H
