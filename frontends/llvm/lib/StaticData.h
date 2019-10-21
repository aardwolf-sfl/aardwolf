#ifndef AARDWOLF_STATIC_DATA_H
#define AARDWOLF_STATIC_DATA_H

#include "llvm/Pass.h"
#include "llvm/Support/raw_ostream.h"

#include "Statement.h"

namespace aardwolf {
class StaticData : public llvm::ModulePass {
public:
    static char ID;

    StaticData() : ModulePass(ID) {}

    virtual bool runOnModule(llvm::Module &M);
    virtual void getAnalysisUsage(llvm::AnalysisUsage &AU) const;
};
}

#endif // AARDWOLF_STATIC_DATA_H
