#include "StatementRepository.h"

#include "Statement.h"

using namespace aardwolf;

uint64_t StatementRepository::getStatementId(Statement &Stmt) {
  auto Found = StmtsIdMap.find(Stmt.Instr);
  if (Found == StmtsIdMap.end()) {
    uint64_t Id = StmtsIdMap.size() + 1;
    StmtsIdMap.insert({Stmt.Instr, Id});
    return Id;
  } else {
    return Found->second;
  }
}

uint64_t StatementRepository::getValueId(const llvm::Value *Value) {
  auto Found = ValuesIdMap.find(Value);
  if (Found == ValuesIdMap.end()) {
    uint64_t Id = ValuesIdMap.size() + 1;
    ValuesIdMap.insert({Value, Id});
    return Id;
  } else {
    return Found->second;
  }
}

uint32_t StatementRepository::getFileId(const std::string Filepath) {
  auto Found = FilesIdMap.find(Filepath);
  if (Found == FilesIdMap.end()) {
    uint64_t Id = FilesIdMap.size() + 1;
    FilesIdMap.insert({Filepath, Id});
    return Id;
  } else {
    return Found->second;
  }
}

void StatementRepository::registerStatement(llvm::Function *F,
                                            Statement &Stmt) {
  InstrStmtMap.insert({ Stmt.Instr, Stmt });

  getStatementId(Stmt);

  for (auto I : Stmt.In) {
    getValueId(I.getValueOrBase());
  }

  if (Stmt.Out != nullptr) {
    getValueId(Stmt.Out->getValueOrBase());
  }

  FuncInstrsMap[F].push_back(Stmt.Instr);
}

void StatementRepository::addSuccessor(llvm::Instruction *Stmt,
                                       llvm::Instruction *Succ) {
  // TODO: Check if they are both already registered.
  InstrSucc[Stmt].push_back(Succ);
}
