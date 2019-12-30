#include "DynamicData.h"
#include "StatementDetection.h"
#include "StaticData.h"

#include <stdlib.h>

#include "llvm/Bitcode/BitcodeWriter.h"
#include "llvm/IRReader/IRReader.h"
#include "llvm/Passes/PassBuilder.h"
#include "llvm/Support/CommandLine.h"
#include "llvm/Support/SourceMgr.h"
#include "llvm/Support/ToolOutputFile.h"
#include "llvm/Support/raw_ostream.h"

using namespace aardwolf;

static llvm::cl::OptionCategory AardwolfCategory{"Aardwolf options"};

static llvm::cl::opt<std::string> InputFilename{
    llvm::cl::Positional,
    llvm::cl::desc{"<input bitcode file>"},
    llvm::cl::value_desc{"bitcode filename"},
    llvm::cl::init(""),
    llvm::cl::Required,
    llvm::cl::cat{AardwolfCategory}};

static llvm::cl::opt<std::string>
    OutputDirectory("o", llvm::cl::desc("Override output directory"),
                    llvm::cl::value_desc("directory name"),
                    llvm::cl::init("aardwolf"),
                    llvm::cl::cat{AardwolfCategory});

static llvm::cl::opt<bool>
    NoInstrumentation("disable-instrumentation",
                      llvm::cl::desc("Do not write instrumented bitcode file"),
                      llvm::cl::cat{AardwolfCategory});

static void process(llvm::Module &M) {
  llvm::ModulePassManager MPM;
  StaticData StaticData(OutputDirectory);
  DynamicData DynamicData;

  MPM.addPass(StaticData);

  if (!NoInstrumentation) {
    MPM.addPass(DynamicData);
  }

  llvm::ModuleAnalysisManager MAM;
  MAM.registerPass([&] { return StatementDetection(); });

  llvm::PassBuilder PB;
  PB.registerModuleAnalyses(MAM);

  MPM.run(M, MAM);
}

int main(int Argc, char **Argv) {
  llvm::cl::HideUnrelatedOptions(AardwolfCategory);

  llvm::cl::ParseCommandLineOptions(
      Argc, Argv,
      "Produces static data files and instruments the program for Aardwolf "
      "analysis\n");

  llvm::llvm_shutdown_obj SDO;

  llvm::SMDiagnostic Err;
  llvm::LLVMContext Ctx;
  std::unique_ptr<llvm::Module> M =
      llvm::parseIRFile(InputFilename.getValue(), Err, Ctx);

  if (!M) {
    llvm::errs() << "Error reading input bitcode file: " << InputFilename
                 << "\n";
    Err.print(Argv[0], llvm::errs());
    return 1;
  }

  std::unique_ptr<llvm::ToolOutputFile> Out;
  std::error_code EC;
  Out.reset(new llvm::ToolOutputFile(OutputDirectory + "/!instrumented.bc", EC,
                                     llvm::sys::fs::OF_None));
  if (EC) {
    llvm::errs() << "Error writing to output directory: " << EC.message()
                 << '\n';
    return 1;
  }

  process(*M);

  if (!NoInstrumentation) {
    Out->keep();
    llvm::WriteBitcodeToFile(*M, Out.get()->os());
  }

  return 0;
}
