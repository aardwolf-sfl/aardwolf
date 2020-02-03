#include "StatementDetection.h"

#include <queue>
#include <unordered_set>

#include "llvm/IR/Instruction.h"
#include "llvm/IR/IntrinsicInst.h"
#include "llvm/IR/Value.h"
#include "llvm/Passes/PassBuilder.h"
#include "llvm/Passes/PassPlugin.h"
#include "llvm/Transforms/Utils/Local.h"

#include "Exceptions.h"
#include "StatementRepository.h"

using namespace aardwolf;

std::shared_ptr<Access> getValueAccess(const llvm::User *U);
std::unordered_set<Access, AccessHasher> findInputs(const llvm::Instruction *I);

// Gets value that corresponds to the base "pointer" of a composite type (the
// array or structure itself).
const llvm::Value *findCompositeBase(const llvm::GetElementPtrInst *GEPI) {
  auto B = GEPI->getOperand(0);

  if (auto I = llvm::dyn_cast<llvm::Instruction>(B)) {
    // Found on first try (this is true for arrays).
    if (llvm::isa<llvm::AllocaInst>(I)) {
      return I;
    }

    // Find the alloca instruction transitively.
    auto Inputs = findInputs(I);
    if (Inputs.size() == 1) {
      return Inputs.begin()->getValueOrBase();
    }

    return nullptr;
  } else if (llvm::isa<llvm::GlobalVariable>(B)) {
    return B;
  }

  return nullptr;
}

// Gets values that determine the access to the composite type (e.g., index,
// field, etc.).
std::vector<Access>
findCompositeAccessors(const llvm::GetElementPtrInst *GEPI) {
  std::vector<Access> Output;

  auto AU =
      llvm::dyn_cast<llvm::User>(GEPI->getOperand(GEPI->getNumOperands() - 1));

  // Try if the accessor is valid access on its own.
  auto A = getValueAccess(AU);

  if (A == nullptr) {
    if (auto C = llvm::dyn_cast<llvm::Constant>(AU)) {
      // Constant. For accessors they are important.
      Output.push_back(Access::makeScalar(C));
    } else if (auto I = llvm::dyn_cast<llvm::Instruction>(AU)) {
      // Find the alloca/method call instructions.
      for (auto Input : findInputs(I)) {
        Output.push_back(Input);
      }
    }
  } else {
    Output.push_back(*A);
  }

  return Output;
}

std::shared_ptr<Access> getValueAccess(const llvm::User *U) {
  if (U == nullptr) {
    return nullptr;
  } else if (llvm::isa<llvm::AllocaInst>(U)) {
    // Local variable.
    return std::make_shared<Access>(Access::makeScalar(U));
  } else if (llvm::isa<llvm::CallInst>(U)) {
    // Result of a function call.
    return std::make_shared<Access>(Access::makeScalar(U));
  } else if (auto GV = llvm::dyn_cast<llvm::GlobalVariable>(U)) {
    if (GV->isConstant()) {
      // If isConstant is true, then the value is immutable throughout the
      // execution, therefore we do not treat such values as variables.
      return nullptr;
    }

    // Global variable.
    return std::make_shared<Access>(Access::makeScalar(GV));
  } else if (auto GEPI = llvm::dyn_cast<llvm::GetElementPtrInst>(U)) {
    auto B = findCompositeBase(GEPI);
    auto A = findCompositeAccessors(GEPI);

    if (B == nullptr || A.empty()) {
      llvm::errs() << "TODO: invalid state\n";
      return nullptr;
    }

    // Struct pointer is special for us, all other are treated as general
    // pointers.
    if (GEPI->getSourceElementType()->isStructTy()) {
      return std::make_shared<Access>(Access::makeStructural(B, *A.begin()));
    } else {
      return std::make_shared<Access>(Access::makeArrayLike(B, A));
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
std::unordered_set<Access, AccessHasher>
findInputs(const llvm::Instruction *I) {
  std::unordered_set<Access, AccessHasher> Result(16);

  // Use BFS-like search backwards in the control flow graph and search for
  // instructions that represent supported values (see Statement.h). While doing
  // it, properly handle "transitive" nodes like loads, arithmetics or
  // conversions which might use the instructions that we are looking for.
  std::queue<const llvm::User *> Q;
  Q.push(I);

  while (!Q.empty()) {
    auto QU = Q.front();
    Q.pop();

    auto V = getValueAccess(QU);
    if (QU != I && V != nullptr) {
      Result.insert(*V);
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
const llvm::DebugLoc getInstrLoc(const llvm::Instruction *I) {
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

const std::string getDebugLocFile(llvm::DebugLoc Loc) {
  if (Loc->getScope()->getDirectory() == "") {
    return Loc->getScope()->getFilename().str();
  } else {
    return (Loc->getScope()->getDirectory() + "/" +
            Loc->getScope()->getFilename())
        .str();
  }
}

// Retrieves the location of the whole statement in the original source code.
const Location getStmtLoc(const Statement &Stmt) {
  auto InstrLoc = getInstrLoc(Stmt.Instr);
  auto Line = InstrLoc.getLine();
  auto Col = InstrLoc.getCol();
  auto File = getDebugLocFile(InstrLoc);

  // We might do some range computations, however, in any cases it will not be
  // possible. About simple statement like `a = 0`, we only have information
  // about location of equal symbol, nothing else.
  return Location(File, LineCol(Line, Col), LineCol(Line, Col));
}

Statement runOnInstr(llvm::Instruction *I) {
  Statement Result;

  if (auto *RI = llvm::dyn_cast<llvm::ReturnInst>(I)) {
    Result.Instr = RI;
    Result.In = findInputs(RI);
    Result.Loc = getStmtLoc(Result);
    return Result;
  }

  if (auto *BI = llvm::dyn_cast<llvm::BranchInst>(I)) {
    if (BI->isConditional()) {
      Result.Instr = BI;
      Result.In = findInputs(BI);
      Result.Loc = getStmtLoc(Result);
      return Result;
    }
  }

  if (auto *SI = llvm::dyn_cast<llvm::SwitchInst>(I)) {
    Result.Instr = SI;
    Result.In = findInputs(SI);
    Result.Loc = getStmtLoc(Result);
    return Result;
  }

  if (auto *II = llvm::dyn_cast<llvm::InvokeInst>(I)) {
    Result.Instr = II;
    Result.In = findInputs(II);
    Result.Loc = getStmtLoc(Result);
    return Result;
  }

  if (auto *SI = llvm::dyn_cast<llvm::StoreInst>(I)) {
    Result.Instr = SI;
    Result.In = findInputs(SI);
    Result.Out = getValueAccess(llvm::dyn_cast<llvm::User>(SI->getOperand(1)));
    Result.Loc = getStmtLoc(Result);
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
    Result.Loc = getStmtLoc(Result);

    if (!CI->getType()->isVoidTy()) {
      Result.Out = std::make_shared<Access>(Access::makeScalar(CI));
    }

    return Result;
  }

  return Result;
}

StatementRepository StatementDetection::run(llvm::Module &M,
                                            llvm::ModuleAnalysisManager &) {
  StatementRepository Repo;

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
          Stmt = runOnInstr(&I);
        } catch (UnknownLocation &) {
          // This statement does not have a location. It can be an instruction
          // that is not present in the source code and is added by the
          // compiler.
          continue;
        }

        // If the instruction represents a valid statement.
        if (Stmt.Instr != nullptr) {
          // Register the statement at this point for user-friendly identifiers
          // that follow the order of occurrence of the statement in the source
          // code.
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

  return Repo;
}

llvm::AnalysisKey StatementDetection::Key;
