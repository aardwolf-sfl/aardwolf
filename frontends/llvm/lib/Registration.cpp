#include "DynamicData.h"
#include "StatementDetection.h"
#include "StaticData.h"

#include "llvm/Passes/PassBuilder.h"
#include "llvm/Passes/PassPlugin.h"

using namespace aardwolf;

// Registration for llvm tools
llvm::PassPluginLibraryInfo getAardwolfPluginInfo() {
  return {LLVM_PLUGIN_API_VERSION, "aardwolf-statement-detection",
          LLVM_VERSION_STRING, [](llvm::PassBuilder &PB) {
            std::string DestDir(std::getenv("AARDWOLF_DATA_DEST"));

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
