#ifndef AARDWOLF_STATEMENT_REPOSITORY_H
#define AARDWOLF_STATEMENT_REPOSITORY_H

#include <map>
#include <unordered_map>

#include "llvm/IR/DebugInfoMetadata.h"
#include "llvm/IR/Function.h"
#include "llvm/IR/Instructions.h"
#include "llvm/IR/Value.h"

#include "Statement.h"

namespace aardwolf {

struct StatementRepository {
public:
  // Mapping from llvm instruction to aardwolf statement.
  std::map<llvm::Instruction *, Statement> InstrStmtMap;

  // Mapping from function to list of aardwolf statements
  // (represented by internal llvm instructions).
  std::map<llvm::Function *, std::vector<llvm::Instruction *>> FuncInstrsMap;

  // All successors of each statement (represented by internal llvm
  // instruction).
  std::map<llvm::Instruction *, std::vector<llvm::Instruction *>> InstrSucc;

  // Mapping from aardwolf statements (represented by llvm instructions
  // themselves) to assigned numeric id.
  std::unordered_map<const llvm::Instruction *, std::pair<uint64_t, uint64_t>> StmtsIdMap;

  // Mapping from llvm values (used for variables) to assigned numeric id.
  std::unordered_map<const llvm::Value *, uint64_t> ValuesIdMap;

  // Mapping from filenames in analysed module to assigned numeric id.
  std::map<const std::string, uint64_t> FilesIdMap;

  // TODO: Mappings: Function names to statements (for function-level
  // granularity).

  // Registers the statement and assigns it and its values a numeric id.
  void registerStatement(llvm::Function *F, Statement &Stmt);

  // Registers Succ as the successor of Stmt. The arguments are llvm
  // instructions behind the statements. Both Stmt and Succ must be already
  // registered.
  void addSuccessor(llvm::Instruction *Stmt, llvm::Instruction *Succ);

  std::pair<uint64_t, uint64_t> getStatementId(Statement &Stmt);
  uint64_t getValueId(const llvm::Value *Value);
  uint64_t getFileId(const std::string &File);
};
} // namespace aardwolf

#endif // AARDWOLF_STATEMENT_REPOSITORY_H
