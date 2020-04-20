#include "DynamicData.h"
#include "StatementDetection.h"
#include "StaticData.h"

#include "llvm/IR/LegacyPassManager.h"
#include "llvm/Passes/PassBuilder.h"
#include "llvm/Passes/PassPlugin.h"
#include "llvm/Transforms/IPO/PassManagerBuilder.h"

using namespace aardwolf;

std::string getDestDir() {
  std::string DestDir;
  auto DestDirEnv = std::getenv("AARDWOLF_DATA_DEST");

  if (DestDirEnv != nullptr) {
    DestDir = DestDirEnv;
  }
  return DestDir;
}

// Registration for llvm tools
llvm::PassPluginLibraryInfo getAardwolfPluginInfo() {
  return {LLVM_PLUGIN_API_VERSION, "aardwolf-llvm", LLVM_VERSION_STRING,
          [](llvm::PassBuilder &PB) {
            auto DestDir = getDestDir();

            PB.registerAnalysisRegistrationCallback(
                [](llvm::ModuleAnalysisManager &MAM) {
                  MAM.registerPass([&] { return StatementDetection(); });
                });

            PB.registerPipelineParsingCallback(
                [&DestDir](llvm::StringRef Name, llvm::ModulePassManager &MPM,
                           llvm::ArrayRef<llvm::PassBuilder::PipelineElement>) {
                  if (Name == "aardwolf-static-data") {
                    MPM.addPass(StaticData(DestDir));
                    return true;
                  }
                  return false;
                });

            PB.registerPipelineParsingCallback(
                [](llvm::StringRef Name, llvm::ModulePassManager &MPM,
                   llvm::ArrayRef<llvm::PassBuilder::PipelineElement>) {
                  if (Name == "aardwolf-dynamic-data") {
                    MPM.addPass(DynamicData());
                    return true;
                  }
                  return false;
                });
          }};
}

extern "C" LLVM_ATTRIBUTE_WEAK ::llvm::PassPluginLibraryInfo
llvmGetPassPluginInfo() {
  return getAardwolfPluginInfo();
}

// Legacy passes registration in order to be able to use it with clang directly.
// Should be superseded by
// http://llvm.org/docs/WritingAnLLVMPass.html#building-pass-plugins in the
// future.
char LegacyStatementDetection::ID = 0;
static llvm::RegisterPass<LegacyStatementDetection>
    XStatementDetection("aardwolf-legacy-statement-detection",
                        "Aardwolf Legacy Statement Detection Pass");

char LegacyStaticData::ID = 0;
static llvm::RegisterPass<LegacyStaticData>
    XStaticData("aardwolf-legacy-static-data",
                "Aardwolf Legacy Static Data Pass");

char LegacyDynamicData::ID = 0;
static llvm::RegisterPass<LegacyDynamicData>
    XDynamicData("aardwolf-legacy-dynamic-data",
                 "Aardwolf Legacy Dynamic Data Pass");

static void registerLegacy(const llvm::PassManagerBuilder &,
                           llvm::legacy::PassManagerBase &PM) {
  auto DestDir = getDestDir();

  PM.add(new LegacyStatementDetection());
  PM.add(new LegacyStaticData(DestDir));
  PM.add(new LegacyDynamicData());
}

static llvm::RegisterStandardPasses
    RegisterInjectFuncCall(llvm::PassManagerBuilder::EP_EnabledOnOptLevel0,
                           registerLegacy);
