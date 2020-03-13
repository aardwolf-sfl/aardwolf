#include "StatementRepository.h"

#include "Statement.h"
#include "Tools.h"

using namespace aardwolf;

std::pair<uint64_t, uint64_t>
StatementRepository::getStatementId(Statement &Stmt) {
  auto Found = StmtsIdMap.find(Stmt.Instr);
  if (Found == StmtsIdMap.end()) {
    auto FileId = getFileId(getDebugLocFile(getInstrLoc(Stmt.Instr)));
    uint64_t StmtId = StmtsIdMap.size() + 1;
    auto Id = std::make_pair(FileId, StmtId);
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

uint64_t StatementRepository::getFileId(const std::string &File) {
  auto Found = FilesIdMap.find(File);
  if (Found == FilesIdMap.end()) {
    auto Id = getFileUniqueId(File);
    FilesIdMap.insert({File, Id});
    return Id;
  } else {
    return Found->second;
  }
}

void StatementRepository::registerStatement(llvm::Function *F,
                                            Statement &Stmt) {
  InstrStmtMap.insert({Stmt.Instr, Stmt});

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
