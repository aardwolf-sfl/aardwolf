#include "DynamicData.h"

#include "llvm/IR/IRBuilder.h"
#include "llvm/IR/Instructions.h"
#include "llvm/IR/Module.h"

#include "Statement.h"
#include "StatementDetection.h"
#include "StatementRepository.h"

using namespace aardwolf;

// Dedicated function if we change the type (size) in the future.
llvm::IntegerType *getStmtRefTy(llvm::LLVMContext &Ctx) {
  return llvm::Type::getInt64Ty(Ctx);
}

llvm::FunctionCallee getWriteStmtTracer(llvm::Module &M) {
  auto &Ctx = M.getContext();

  auto VoidTy = llvm::Type::getVoidTy(Ctx);
  auto StmtRefTy = getStmtRefTy(Ctx);

  std::vector<llvm::Type *> WriteParams;
  WriteParams.push_back(StmtRefTy);

  auto WriteStmtTy = llvm::FunctionType::get(VoidTy, WriteParams, false);
  return M.getOrInsertFunction("aardwolf_write_statement", WriteStmtTy);
}

llvm::Value *getVarValue(llvm::Instruction *I) {
  if (auto SI = llvm::dyn_cast<llvm::StoreInst>(I)) {
    return SI->getOperand(0);
  } else if (auto CI = llvm::dyn_cast<llvm::CallBase>(I)) {
    if (CI->getType()->isVoidTy()) {
      return nullptr;
    } else {
      return CI;
    }
  } else {
    return nullptr;
  }
}

std::optional<std::pair<llvm::FunctionCallee, std::vector<llvm::Value *>>>
getDefVarTracer(llvm::Module &M, llvm::Instruction *I) {
  auto &Ctx = M.getContext();

  std::vector<llvm::Type *> Params;
  std::vector<llvm::Value *> Args;
  std::string Name;

  auto Value = getVarValue(I);

  if (Value == nullptr) {
    return std::nullopt;
  }

  auto ValueTy = Value->getType();

  // Since LLVM does not distinguish between signed and unsigned on the type
  // level (only in the instruction level if necessary), we do not do so as
  // well.
  if (ValueTy->isIntegerTy(8)) {
    Name = "aardwolf_write_data_i8";
  } else if (ValueTy->isIntegerTy(16)) {
    Name = "aardwolf_write_data_i16";
  } else if (ValueTy->isIntegerTy(32)) {
    Name = "aardwolf_write_data_i32";
  } else if (ValueTy->isIntegerTy(64)) {
    Name = "aardwolf_write_data_i64";
  } else if (ValueTy->isFloatTy()) {
    Name = "aardwolf_write_data_f32";
  } else if (ValueTy->isDoubleTy()) {
    Name = "aardwolf_write_data_f64";
  }

  if (Name.empty()) {
    Name = "aardwolf_write_data_unsupported";
  } else {
    Params.push_back(ValueTy);
    Args.push_back(Value);
  }

  auto VoidTy = llvm::Type::getVoidTy(Ctx);
  auto TraceTy = llvm::FunctionType::get(VoidTy, Params, false);

  return std::optional<
      std::pair<llvm::FunctionCallee, std::vector<llvm::Value *>>>(
      std::make_pair(M.getOrInsertFunction(Name, TraceTy), Args));
}

bool DynamicDataBase::runBase(llvm::Module &M, StatementRepository &Repo) {
  auto &Ctx = M.getContext();
  auto StmtRefTy = getStmtRefTy(Ctx);

  auto WriteStmt = getWriteStmtTracer(M);

  llvm::IRBuilder<> Builder(Ctx);

  for (auto &F : M) {
    if (F.isDeclaration()) {
      continue;
    }

    for (auto I : Repo.FuncInstrsMap[&F]) {
      std::vector<llvm::Value *> Args;
      Args.push_back(llvm::ConstantInt::get(
          StmtRefTy, Repo.getStatementId(Repo.InstrStmtMap[I])));

      auto CI = Builder.CreateCall(WriteStmt, Args);
      // Instruction can be a terminator, we need to put the printing statement
      // before it.
      CI->insertBefore(I);

      auto WriteVarOptional = getDefVarTracer(M, I);
      if (WriteVarOptional.has_value()) {
        if (Repo.InstrStmtMap[I].Out == nullptr) {
          // TODO: Invalid var trace.
        }

        auto WriteVar = WriteVarOptional.value();
        auto CI = Builder.CreateCall(WriteVar.first, WriteVar.second);
        // Instruction is a store or function call, it is not a terminator.
        // Moreover, in case of function call, it is the value returned by the
        // function, therefore, it needs to be printed *after* the execution of
        // the instruction.
        CI->insertAfter(I);
      } else if (Repo.InstrStmtMap[I].Out == nullptr) {
        // TODO: Forgotten var trace.
      }
    }
  }

  return true;
}

llvm::PreservedAnalyses DynamicData::run(llvm::Module &M,
                                         llvm::ModuleAnalysisManager &MAM) {
  if (runBase(M, MAM.getResult<StatementDetection>(M))) {
    return llvm::PreservedAnalyses::none();
  } else {
    return llvm::PreservedAnalyses::all();
  }
}

LegacyDynamicData::LegacyDynamicData() : llvm::ModulePass(ID) {}

bool LegacyDynamicData::runOnModule(llvm::Module &M) {
  return runBase(M, getAnalysis<LegacyStatementDetection>().Repo);
}

void LegacyDynamicData::getAnalysisUsage(llvm::AnalysisUsage &AU) const {
  AU.setPreservesAll();
  AU.addRequired<LegacyStatementDetection>();
}
