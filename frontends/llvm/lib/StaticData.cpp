#include "StaticData.h"

#include <cstdint>
#include <cstdlib>

#include "llvm/IR/Module.h"
#include "llvm/Pass.h"
#include "llvm/Passes/PassBuilder.h"
#include "llvm/Passes/PassPlugin.h"
#include "llvm/Support/raw_ostream.h"

#include "Statement.h"
#include "StatementDetection.h"
#include "StatementRepository.h"

using namespace aardwolf;

#define TOKEN_STATEMENT 0xff
#define TOKEN_FUNCTION 0xfe
#define TOKEN_FILENAMES 0xfd

#define TOKEN_VALUE_SCALAR 0xe0
#define TOKEN_VALUE_STRUCTURAL 0xe1
#define TOKEN_VALUE_ARRAY_LIKE 0xe2

#define META_ARG 0x61
#define META_RET 0x62
#define META_CALL 0x64

// TODO: Is there an idiomatic C++ way how to do these writeBytes functions?
void writeBytes(llvm::raw_ostream &Stream, uint8_t value) {
  Stream.write((const char *)&value, sizeof(uint8_t));
}

void writeBytes(llvm::raw_ostream &Stream, uint32_t value) {
  Stream.write((const char *)&value, sizeof(uint32_t));
}

void writeBytes(llvm::raw_ostream &Stream, uint64_t value) {
  Stream.write((const char *)&value, sizeof(uint64_t));
}

void writeBytes(llvm::raw_ostream &Stream, std::string value) {
  Stream.write(value.c_str(), sizeof(char) * value.size());
  Stream.write(0);
}

void exportFunctionName(llvm::raw_ostream &Stream, llvm::Function &F) {
  writeBytes(Stream, (uint8_t)TOKEN_FUNCTION);
  writeBytes(Stream, F.getName().str());
}

void exportAccess(StatementRepository &Repo, llvm::raw_ostream &Stream,
                  const Access *Access) {
  if (Access->isScalar()) {
    writeBytes(Stream, (uint8_t)TOKEN_VALUE_SCALAR);
    writeBytes(Stream, Repo.getValueId(Access->getValue()));
  } else {
    if (Access->getType() == AccessType::Structural) {
      writeBytes(Stream, (uint8_t)TOKEN_VALUE_STRUCTURAL);
      // writeBytes(Stream, Repo.getValueId(Access->getBase()));
      exportAccess(Repo, Stream, &Access->getBase());
      exportAccess(Repo, Stream, &Access->getAccessors()[0]);
    } else if (Access->getType() == AccessType::ArrayLike) {
      writeBytes(Stream, (uint8_t)TOKEN_VALUE_ARRAY_LIKE);
      exportAccess(Repo, Stream, &Access->getBase());
      writeBytes(Stream, (uint32_t)Access->getAccessors().size());
      for (auto Var : Access->getAccessors()) {
        exportAccess(Repo, Stream, &Var);
      }
    }
  }
}

uint8_t getMetadata(Statement &Stmt) {
  uint8_t metadata = 0;

  if (Stmt.isArg()) {
    metadata |= META_ARG;
  }

  if (Stmt.isRet()) {
    metadata |= META_RET;
  }

  if (Stmt.isCall()) {
    metadata |= META_CALL;
  }

  return metadata;
}

void exportStatement(StatementRepository &Repo, llvm::raw_ostream &Stream,
                     Statement &Stmt, std::vector<Statement *> &Successors) {
  // Statement id.
  writeBytes(Stream, (uint8_t)TOKEN_STATEMENT);
  writeBytes(Stream, Repo.getStatementId(Stmt));

  // Successors.
  writeBytes(Stream, (uint8_t)Successors.size());

  for (auto Succ : Successors) {
    writeBytes(Stream, Repo.getStatementId(*Succ));
  }

  // Defs.
  if (Stmt.Out != nullptr) {
    writeBytes(Stream, (uint8_t)1);
    exportAccess(Repo, Stream, Stmt.Out.get());
  } else {
    writeBytes(Stream, (uint8_t)0);
  }

  // Uses.
  writeBytes(Stream, (uint8_t)Stmt.In.size());

  for (auto Use : Stmt.In) {
    exportAccess(Repo, Stream, &Use);
  }

  // Location.
  writeBytes(Stream, Repo.getFileId(Stmt.Loc.File));
  writeBytes(Stream, (uint32_t)Stmt.Loc.Begin.Line);
  writeBytes(Stream, (uint32_t)Stmt.Loc.Begin.Col);
  writeBytes(Stream, (uint32_t)Stmt.Loc.End.Line);
  writeBytes(Stream, (uint32_t)Stmt.Loc.End.Col);

  // Statement metadata
  writeBytes(Stream, getMetadata(Stmt));
}

void exportMetadata(StatementRepository &Repo, llvm::raw_ostream &Stream) {
  writeBytes(Stream, (uint8_t)TOKEN_FILENAMES);
  writeBytes(Stream, (uint32_t)Repo.FilesIdMap.size());

  for (auto F : Repo.FilesIdMap) {
    writeBytes(Stream, F.second);
    writeBytes(Stream, F.first);
  }
}

std::string getFilename(std::string Name) {
  auto SepPos = Name.rfind('/');

  if (SepPos != std::string::npos) {
    return Name.substr(SepPos + 1, Name.size() - 1);
  } else {
    return Name;
  }
}

StaticDataBase::StaticDataBase() {}

StaticDataBase::StaticDataBase(std::string &DestDir) : DestDir(DestDir) {}

bool StaticDataBase::runBase(llvm::Module &M, StatementRepository &Repo) {
  std::string Dest;

  if (!DestDir.empty()) {
    Dest = DestDir + '/';
  }

  std::string Filename = (Dest + getFilename(M.getName().str()) + ".aard");
  std::error_code EC;
  llvm::raw_fd_ostream Stream(llvm::StringRef(Filename), EC);

  if (EC) {
    llvm::errs() << EC.message() << "\n";
    return false;
  }

  // Header.
  Stream << "AARD/S1";

  std::vector<Statement *> Outgoing;

  for (auto &F : M) {
    if (F.isDeclaration()) {
      continue;
    }

    exportFunctionName(Stream, F);

    for (auto &BB : F) {
      for (auto &I : BB) {
        auto Stmt = Repo.InstrStmtMap.find(&I);

        if (Stmt != Repo.InstrStmtMap.end()) {
          for (auto Succ : Repo.InstrSucc[Stmt->first]) {
            Outgoing.push_back(&Repo.InstrStmtMap[Succ]);
          }

          exportStatement(Repo, Stream, Stmt->second, Outgoing);
          Outgoing.clear();
        }
      }
    }
  }

  exportMetadata(Repo, Stream);

  return false;
}

StaticData::StaticData(std::string &DestDir) : StaticDataBase(DestDir) {}

llvm::PreservedAnalyses StaticData::run(llvm::Module &M,
                                        llvm::ModuleAnalysisManager &MAM) {
  if (runBase(M, MAM.getResult<StatementDetection>(M))) {
    return llvm::PreservedAnalyses::none();
  } else {
    return llvm::PreservedAnalyses::all();
  }
}

LegacyStaticData::LegacyStaticData() : llvm::ModulePass(ID) {}

LegacyStaticData::LegacyStaticData(std::string &DestDir)
    : llvm::ModulePass(ID), StaticDataBase(DestDir) {}

bool LegacyStaticData::runOnModule(llvm::Module &M) {
  return runBase(M, getAnalysis<LegacyStatementDetection>().Repo);
}

void LegacyStaticData::getAnalysisUsage(llvm::AnalysisUsage &AU) const {
  AU.setPreservesAll();
  AU.addRequired<LegacyStatementDetection>();
}
