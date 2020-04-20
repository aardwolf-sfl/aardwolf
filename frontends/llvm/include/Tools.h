#ifndef AARDWOLF_TOOLS_H
#define AARDWOLF_TOOLS_H

#include "llvm/IR/DebugInfoMetadata.h"
#include "llvm/IR/DebugLoc.h"
#include "llvm/IR/IntrinsicInst.h"
#include "llvm/Transforms/Utils/Local.h"

namespace aardwolf {

const llvm::DebugLoc getInstrLoc(const llvm::Instruction *I);
const std::string getDebugLocFile(llvm::DebugLoc Loc);
uint64_t getFileUniqueId(const std::string &File);

} // namespace aardwolf

#endif // AARDWOLF_TOOLS_H
