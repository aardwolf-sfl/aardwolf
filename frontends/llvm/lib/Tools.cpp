#include "Tools.h"

#include "llvm/IR/DebugInfoMetadata.h"
#include "llvm/IR/DebugLoc.h"
#include "llvm/IR/IntrinsicInst.h"
#include "llvm/Transforms/Utils/Local.h"

#include "Exceptions.h"

using namespace aardwolf;

// Retrieves the instruction location in the original source code. If this data
// is not available, it throws an UnknownLocation exception.
const llvm::DebugLoc aardwolf::getInstrLoc(const llvm::Instruction *I) {
  if (auto Loc = I->getDebugLoc()) {
    if (Loc->getScope() != nullptr) {
      return Loc;
    }
  } else if (llvm::isa<llvm::StoreInst>(I) &&
             llvm::isa<llvm::Argument>(I->getOperand(0))) {
    // Function argument.
    auto Alloca = I->getOperand(1);

    // NOTE: Can there be multiple debug uses?
    for (auto Dbg : llvm::FindDbgAddrUses(Alloca)) {
      auto Loc = Dbg->getDebugLoc();
      if (Loc->getScope() != nullptr) {
        return Loc;
      }
    }
  }

  throw UnknownLocation();
}

const std::string aardwolf::getDebugLocFile(llvm::DebugLoc Loc) {
  if (Loc->getScope()->getDirectory() == "") {
    return Loc->getScope()->getFilename().str();
  } else {
    return (Loc->getScope()->getDirectory() + "/" +
            Loc->getScope()->getFilename())
        .str();
  }
}

#ifdef _WIN32
uint64_t aardwolf::getFileUniqueId(const std::string &File) {
  // TODO
  // * https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfileinformationbyhandle
  // * https://docs.microsoft.com/en-us/windows/win32/api/fileapi/ns-fileapi-by_handle_file_information (nFileIndexLow, nFileIndexHigh)
  return 0;
}
#endif

#ifdef unix
#include <sys/stat.h>
#include <sys/types.h>
#include <unistd.h>

uint64_t aardwolf::getFileUniqueId(const std::string &File) {
  struct stat statbuf;
  int status = stat(File.c_str(), &statbuf);

  if (status == 0) {
    return (uint64_t)statbuf.st_ino;
  } else {
    // TODO: Raise error.
    return 0;
  }
}
#endif
