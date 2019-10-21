#include "StatementDetection.h"

#include <set>
#include <queue>

#include "llvm/IR/Value.h"
#include "llvm/IR/Instruction.h"
#include "llvm/IR/IntrinsicInst.h"
#include "llvm/Transforms/Utils/Local.h"

#include "Exceptions.h"

using namespace aardwolf;

// Finds inputs of an instruction which are then used as inputs
// in the Statement structure. The values it returns are these llvm
// instructions:
//
//   * AllocaInst - represents a local variable.
//   * CallInst - represents the result of a function call.
//   * GlobalValue - represents a global variable.
//
// It also properly handles pointer accesses - it adds both the pointer and
// the offset variables/values.
//
// If given instruction is StoreInst, the resulting set does not contain
// the destination variable. If the destination variable is a pointer,
// it does include the accessor (e.g., offset) value.
std::set<const llvm::Value*> findInputs(const llvm::Instruction *I) {
    std::set<const llvm::Value*> Result;

    // Use BFS-like search backwards in the control flow graph and search for
    // AllocaInst and CallInst. While doing it, properly handle "transitive"
    // nodes like GetElementPtrInst which might use the instructions that
    // we are looking for.
    std::queue<const llvm::Instruction*> Q;
    Q.push(I);

    while (!Q.empty()) {
        auto QI = Q.front();
        Q.pop();

        if (llvm::isa<llvm::AllocaInst>(QI) && QI != I) {
            // Local variable.
            Result.insert(QI);
        } else if (llvm::isa<llvm::CallInst>(QI) && QI != I) {
            // Result of a function call.
            Result.insert(QI);
        } else if (auto *SI = llvm::dyn_cast<llvm::StoreInst>(QI)) {
            // If the instruction is StoreInst, we must not to include the destination variable.
            if (auto *In = llvm::dyn_cast<llvm::Instruction>(SI->getOperand(0))) {
                Q.push(In);
            }

            // However, in case of pointers, we must include the accessor values.
            if (auto *GEPI = llvm::dyn_cast<llvm::GetElementPtrInst>(SI->getOperand(1))) {
                for (auto &Idx : GEPI->indices()) {
                    if (auto *In = llvm::dyn_cast<llvm::Instruction>(Idx)) {
                        Q.push(In);
                    }
                }
            }
        } else {
            // Add all operands as neighbors into the queue.
            for (const llvm::Use &U : QI->operands()) {
                if (auto *In = llvm::dyn_cast<llvm::Instruction>(U)) {
                    Q.push(In);
                } else if (llvm::isa<llvm::GlobalValue>(U)) {
                    // Global variable.
                    Result.insert(U);
                }
            }
        }
    }

    return Result;
}

// Finds the variable into which the StoreInst assigns the value. The output
// is one of these llvm classes:
//
//   * AllocaInst - represents a local variable.
//   * GlobalValue - represents a global variable.
//
// Pointers are treated such that only the pointer base address variable
// is returned, not the accessor (e.g., offset) values, because these are
// rather the inputs of the instruction (just specifying the precise location,
// but the contents of the pointer is what is actually changed).
const llvm::Value* findStoreDest(const llvm::StoreInst *SI) {
    auto Dest = SI->getOperand(1);

    if (llvm::isa<llvm::AllocaInst>(Dest)) {
        // Local variable.
        return Dest;
    } else if (auto *GEPI = llvm::dyn_cast<llvm::GetElementPtrInst>(Dest)) {
        // Pointer variable (array, etc.).
        auto Inputs = findInputs(llvm::dyn_cast<llvm::Instruction>(GEPI->getOperand(0)));

        for (auto Input : Inputs) {
            if (llvm::isa<llvm::AllocaInst>(Input)) {
                return Input;
            } else if (llvm::isa<llvm::GlobalValue>(Input)) {
                return Input;
            }
        }

        return nullptr;
    } else if (llvm::isa<llvm::GlobalValue>(Dest)) {
        // Global variable.
        return Dest;
    } else {
        return nullptr;
    }
}

// Retrieves the instruction location in the original source code. If this data
// is not available, it throws an UnknownLocation exception.
const llvm::DebugLoc getDebugLoc(const llvm::Instruction *I) {
    if (auto Loc = I->getDebugLoc()) {
        if (Loc->getScope() != nullptr/* && llvm::isa<llvm::DIScope>(Loc->getScope())*/) {
            return Loc;
        }
    } else if (llvm::isa<llvm::StoreInst>(I) && llvm::isa<llvm::Argument>(I->getOperand(0))) {
        // Function argument.
        auto Alloca = I->getOperand(1);

        // NOTE: Can there be multiple debug uses?
        for (auto Dbg : llvm::FindDbgAddrUses(Alloca)) {
            auto Loc = Dbg->getDebugLoc();
            // llvm::isa_and_nonnull is available since LLVM 9.0
            if (Loc->getScope() != nullptr/* && llvm::isa<llvm::DIScope>(Loc->getScope())*/) {
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
        Result.Out = findStoreDest(SI);
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

        if (!CI->doesNotReturn()) {
            Result.Out = CI;
        }

        return Result;
    }

    return Result;
}

bool StatementDetection::runOnModule(llvm::Module &M) {
    // First and last statements for each non-empty basic block.
    std::map<const llvm::BasicBlock*, std::pair<llvm::Instruction*, llvm::Instruction*>> BBBounds;

    for (auto &F : M) {
        if (F.isDeclaration()) {
            // Only functions that are defined.
            continue;
        }

        // First, detect all statements in the function.
        for (auto &BB : F) {
            // Store the first detected statement for proper successor chaining between basic blocks.
            llvm::Instruction *First = nullptr;
            // Store previous detected statement for chaining statements.
            llvm::Instruction *Prev = nullptr;

            for (auto &I : BB) {
                Statement Stmt;
                try {
                    Stmt = runOnInstruction(&I);
                } catch (UnknownLocation&) {
                    // TODO: There might be some special instructions with no debug info
                    //       (e.g., `store i32 0, i32* %1, align 4` in main function).
                    continue;
                }

                // If the instruction represents a valid statement.
                if (Stmt.Instr != nullptr) {
                    // Add the mapping from llvm instruction to the statement.
                    Repo.StmtMap.insert({ Stmt.Instr, Stmt });
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

            std::queue<llvm::BasicBlock*> BBPred;

            for (auto It = llvm::pred_begin(&BB), Et = llvm::pred_end(&BB); It != Et; ++It) {
                // Add all predecessors by default.
                BBPred.push(*It);
            }

            while (!BBPred.empty()) {
                auto P = BBPred.front();
                BBPred.pop();
                auto PredFound = BBBounds.find(P);

                if (PredFound == BBBounds.end()) {
                    // If the predecessor basic block is empty, add also its predecessor to the queue.
                    // That is, we need to find all non-empty predecessors of the basic block.
                    for (auto It = llvm::pred_begin(P), Et = llvm::pred_end(P); It != Et; ++It) {
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
static llvm::RegisterPass<StatementDetection> X("aard-stmt-detection", "Aardwolf Statement Detection Pass");