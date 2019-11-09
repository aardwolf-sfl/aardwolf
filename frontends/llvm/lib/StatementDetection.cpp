#include "StatementDetection.h"

#include <queue>
#include <set>

#include "llvm/IR/Instruction.h"
#include "llvm/IR/IntrinsicInst.h"
#include "llvm/IR/Value.h"
#include "llvm/Transforms/Utils/Local.h"

#include "Exceptions.h"

using namespace aardwolf;

std::shared_ptr<Value> getValue(const llvm::User *U);
std::set<std::shared_ptr<Value>> findInputs(const llvm::Instruction *I);

const llvm::Value *getCompositeBase(const llvm::GetElementPtrInst *GEPI) {
  if (auto I = llvm::dyn_cast<llvm::Instruction>(GEPI->getOperand(0))) {
    // Found on first try (this applies for arrays).
    if (llvm::isa<llvm::AllocaInst>(I)) {
      return I;
    }

    // Find the alloca instruction transitively.
    auto Inputs = findInputs(I);
    if (Inputs.size() == 1) {
      return Inputs.begin()->get()->Base;
    }

    return nullptr;
  }

  return nullptr;
}

std::shared_ptr<Value>
getCompositeAccessor(const llvm::GetElementPtrInst *GEPI) {
  auto AU =
      llvm::dyn_cast<llvm::User>(GEPI->getOperand(GEPI->getNumOperands() - 1));

  // Try if the accessor is value on its own.
  auto A = getValue(AU);

  if (A == nullptr) {
    if (auto C = llvm::dyn_cast<llvm::Constant>(AU)) {
      // Constant. For pointers accessors they are important.
      return Value::Scalar(C);
    } else if (auto I = llvm::dyn_cast<llvm::Instruction>(AU)) {
      // Find the alloca instruction representing the variable.
      auto Inputs = findInputs(I);
      if (Inputs.size() == 1) {
        return *Inputs.begin();
      }
    }

    return nullptr;
  } else {
    return A;
  }
}

std::shared_ptr<Value> getValue(const llvm::User *U) {
  if (U == nullptr) {
    return nullptr;
  } else if (llvm::isa<llvm::AllocaInst>(U)) {
    // Local variable.
    return Value::Scalar(U);
  } else if (llvm::isa<llvm::CallInst>(U)) {
    // Result of a function call.
    return Value::Scalar(U);
  } else if (auto GV = llvm::dyn_cast<llvm::GlobalVariable>(U)) {
    if (GV->isConstant()) {
      // If isConstant is true, then the value is immutable throughout the
      // execution, therefore we do not treat such values as variables.
      return nullptr;
    }

    // Global variable.
    return Value::Scalar(U);
  } else if (auto GEPI = llvm::dyn_cast<llvm::GetElementPtrInst>(U)) {
    auto B = getCompositeBase(GEPI);
    auto A = getCompositeAccessor(GEPI);

    if (B == nullptr || A == nullptr) {
      llvm::errs() << "TODO: invalid state\n";
      return nullptr;
    }

    // TODO: Implementation so far marks `i` and `i + 1` as identical values,
    //       but they are obviously not.
    //       This could lead to invalid assumptions during localization.

    // Struct pointer is special for us, all other are treated as general
    // pointers.
    if (GEPI->getSourceElementType()->isStructTy()) {
      return Value::Struct(B, A);
    } else {
      return Value::Pointer(B, A);
    }
  } else {
    return nullptr;
  }
}

// Finds inputs of an instruction which are then used as inputs
// in the Statement structure.
//
// If given instruction is StoreInst, the resulting set does not contain
// the destination variable.
std::set<std::shared_ptr<Value>> findInputs(const llvm::Instruction *I) {
  std::set<std::shared_ptr<Value>> Result;

  // Use BFS-like search backwards in the control flow graph and search for
  // instructions that represent supported values (see Statement.h). While doing
  // it, properly handle "transitive" nodes like loads, arithmetics or
  // conversions which might use the instructions that we are looking for.
  std::queue<const llvm::User *> Q;
  Q.push(I);

  while (!Q.empty()) {
    auto QU = Q.front();
    Q.pop();

    auto V = getValue(QU);
    if (QU != I && V != nullptr) {
      Result.insert(V);
    } else if (auto *SI = llvm::dyn_cast<llvm::StoreInst>(QU)) {
      // If the instruction is StoreInst, we must not to include the destination
      // variable.
      if (auto *In = llvm::dyn_cast<llvm::User>(SI->getOperand(0))) {
        Q.push(In);
      }
    } else {
      // Add all operands as neighbors into the queue.
      for (const llvm::Use &U : QU->operands()) {
        if (auto *In = llvm::dyn_cast<llvm::User>(U)) {
          Q.push(In);
        }
      }
    }
  }

  return Result;
}

// Retrieves the instruction location in the original source code. If this data
// is not available, it throws an UnknownLocation exception.
const llvm::DebugLoc getDebugLoc(const llvm::Instruction *I) {
  if (auto Loc = I->getDebugLoc()) {
    if (Loc->getScope() !=
        nullptr /* && llvm::isa<llvm::DIScope>(Loc->getScope())*/) {
      return Loc;
    }
  } else if (llvm::isa<llvm::StoreInst>(I) &&
             llvm::isa<llvm::Argument>(I->getOperand(0))) {
    // Function argument.
    auto Alloca = I->getOperand(1);

    // NOTE: Can there be multiple debug uses?
    for (auto Dbg : llvm::FindDbgAddrUses(Alloca)) {
      auto Loc = Dbg->getDebugLoc();
      // llvm::isa_and_nonnull is available since LLVM 9.0
      if (Loc->getScope() !=
          nullptr /* && llvm::isa<llvm::DIScope>(Loc->getScope())*/) {
        return Loc;
      }
    }
  }

  throw UnknownLocation();
}

Statement StatementDetection::runOnInstruction(llvm::Instruction *I) const {
  Statement Result;

  if (auto *RI = llvm::dyn_cast<llvm::ReturnInst>(I)) {
    Result.Instr = RI;
    Result.In = findInputs(RI);
    Result.Loc = getDebugLoc(RI);
    return Result;
  }

  if (auto *BI = llvm::dyn_cast<llvm::BranchInst>(I)) {
    if (BI->isConditional()) {
      Result.Instr = BI;
      Result.In = findInputs(BI);
      Result.Loc = getDebugLoc(BI);
      return Result;
    }
  }

  if (auto *SI = llvm::dyn_cast<llvm::SwitchInst>(I)) {
    Result.Instr = SI;
    Result.In = findInputs(SI);
    Result.Loc = getDebugLoc(SI);
    return Result;
  }

  if (auto *II = llvm::dyn_cast<llvm::InvokeInst>(I)) {
    Result.Instr = II;
    Result.In = findInputs(II);
    Result.Loc = getDebugLoc(II);
    return Result;
  }

  if (auto *SI = llvm::dyn_cast<llvm::StoreInst>(I)) {
    Result.Instr = SI;
    Result.In = findInputs(SI);
    Result.Out = getValue(llvm::dyn_cast<llvm::User>(SI->getOperand(1)));
    Result.Loc = getDebugLoc(SI);
    return Result;
  }

  // Filter intrinsic calls before processing function calls
  if (llvm::isa<llvm::IntrinsicInst>(I)) {
    return Result;
  }

  if (auto *CI = llvm::dyn_cast<llvm::CallInst>(I)) {
    auto Inputs = findInputs(CI);
    Result.Instr = CI;
    Result.In = findInputs(CI);
    Result.Loc = getDebugLoc(CI);

    if (!CI->getType()->isVoidTy()) {
      Result.Out = Value::Scalar(CI);
    }

    return Result;
  }

  return Result;
}

bool StatementDetection::runOnModule(llvm::Module &M) {
  // First and last statements for each non-empty basic block.
  std::map<const llvm::BasicBlock *,
           std::pair<llvm::Instruction *, llvm::Instruction *>>
      BBBounds;

  for (auto &F : M) {
    if (F.isDeclaration()) {
      // Only functions that are defined.
      continue;
    }

    // First, detect all statements in the function.
    for (auto &BB : F) {
      // Store the first detected statement for proper successor chaining
      // between basic blocks.
      llvm::Instruction *First = nullptr;
      // Store previous detected statement for chaining statements.
      llvm::Instruction *Prev = nullptr;

      for (auto &I : BB) {
        Statement Stmt;
        try {
          Stmt = runOnInstruction(&I);
        } catch (UnknownLocation &) {
          // TODO: There might be some special instructions with no debug info
          //       (e.g., `store i32 0, i32* %1, align 4` in main function).
          continue;
        }

        // If the instruction represents a valid statement.
        if (Stmt.Instr != nullptr) {
          // Add the mapping from llvm instruction to the statement.
          Repo.StmtMap.insert({Stmt.Instr, Stmt});
          // For user-friendly identifiers to follow the order
          // of occuring of the statement in the source code file.
          Repo.registerStatement(&F, Stmt);

          if (First == nullptr) {
            First = Stmt.Instr;
            Prev = Stmt.Instr;
          } else {
            // Chain the statements.
            Repo.addSuccessor(Prev, Stmt.Instr);
            Prev = Stmt.Instr;
          }
        }
      }

      // Non-empty basic block.
      if (Prev != nullptr) {
        // Store the first
        BBBounds[&BB] = std::make_pair(First, Prev);
      }
    }

    // Chain also statements between the basic blocks.
    for (auto &BB : F) {
      auto BBFound = BBBounds.find(&BB);
      if (BBFound == BBBounds.end()) {
        // If the basic block is empty, ignore it.
        continue;
      }

      std::queue<llvm::BasicBlock *> BBPred;

      for (auto It = llvm::pred_begin(&BB), Et = llvm::pred_end(&BB); It != Et;
           ++It) {
        // Add all predecessors by default.
        BBPred.push(*It);
      }

      while (!BBPred.empty()) {
        auto P = BBPred.front();
        BBPred.pop();
        auto PredFound = BBBounds.find(P);

        if (PredFound == BBBounds.end()) {
          // If the predecessor basic block is empty, add also its predecessor
          // to the queue. That is, we need to find all non-empty predecessors
          // of the basic block.
          for (auto It = llvm::pred_begin(P), Et = llvm::pred_end(P); It != Et;
               ++It) {
            BBPred.push(*It);
          }
        } else {
          // Add the successors between the basic blocks.
          // That is, chain the last statement of the predecessor basic block
          // with the first statement in the current basic block.
          Repo.addSuccessor(PredFound->second.second, BBFound->second.first);
        }
      }
    }
  }

  return false;
}

void StatementDetection::getAnalysisUsage(llvm::AnalysisUsage &AU) const {
  AU.setPreservesAll();
}

char StatementDetection::ID = 0;
static llvm::RegisterPass<StatementDetection>
    X("aard-stmt-detection", "Aardwolf Statement Detection Pass");