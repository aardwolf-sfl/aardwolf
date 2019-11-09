#include "StaticData.h"

#include <cstdint>

#include "llvm/IR/Module.h"
#include "llvm/Support/raw_ostream.h"

#include "StatementDetection.h"
#include "StatementRepository.h"

using namespace aardwolf;

#define TOKEN_STATEMENT 0xff
#define TOKEN_FUNCTION 0xfe
#define TOKEN_FILENAMES 0xfd

std::string getFilepath(llvm::DebugLoc Loc) {
  if (Loc->getScope()->getDirectory() == "") {
    return Loc->getScope()->getFilename().str();
  } else {
    return (Loc->getScope()->getDirectory() + "/" +
            Loc->getScope()->getFilename())
        .str();
  }
}

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
    writeBytes(Stream, Repo.getValueId(Stmt.Out->Base));
  } else {
    writeBytes(Stream, (uint8_t)0);
  }

  // Uses.
  writeBytes(Stream, (uint8_t)Stmt.In.size());

  for (auto Use : Stmt.In) {
    writeBytes(Stream, Repo.getValueId(Use->Base));
  }

  // Location.
  writeBytes(Stream, (uint8_t)3); // file, line, column
  writeBytes(Stream, Repo.getFileId(getFilepath(Stmt.Loc)));
  writeBytes(Stream, (uint32_t)Stmt.Loc.getLine());
  writeBytes(Stream, (uint32_t)Stmt.Loc.getCol());

  // Statement metadata (bitflags)
  // TODO
  writeBytes(Stream, (uint8_t)0);
}

void exportMetadata(StatementRepository &Repo, llvm::raw_ostream &Stream) {
  writeBytes(Stream, (uint8_t)TOKEN_FILENAMES);
  writeBytes(Stream, (uint32_t)Repo.FilesIdMap.size());

  for (auto F : Repo.FilesIdMap) {
    writeBytes(Stream, F.second);
    writeBytes(Stream, F.first);
  }
}

std::string filename(std::string Name) {
  auto SepPos = Name.rfind('/');

  if (SepPos != std::string::npos) {
    return Name.substr(SepPos + 1, Name.size() - 1);
  } else {
    return Name;
  }
}

bool StaticData::runOnModule(llvm::Module &M) {
  std::string Filename = ("aardwolf." + filename(M.getName().str()) + ".data");
  std::error_code EC;
  llvm::raw_fd_ostream Stream(llvm::StringRef(Filename), EC);

  if (EC) {
    llvm::errs() << EC.message() << "\n";
    return false;
  }

  // Header.
  Stream << "AARD/S1";

  std::vector<Statement *> Outgoing;
  auto Repo = getAnalysis<StatementDetection>().Repo;

  for (auto &F : M) {
    if (F.isDeclaration()) {
      continue;
    }

    exportFunctionName(Stream, F);

    for (auto &BB : F) {
      for (auto &I : BB) {
        auto Stmt = Repo.StmtMap.find(&I);

        if (Stmt != Repo.StmtMap.end()) {
          for (auto Succ : Repo.InstrSucc[Stmt->first]) {
            Outgoing.push_back(&Repo.StmtMap[Succ]);
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

void StaticData::getAnalysisUsage(llvm::AnalysisUsage &AU) const {
  AU.setPreservesAll();
  AU.addRequired<StatementDetection>();
}

char StaticData::ID = 0;
static llvm::RegisterPass<StaticData> X("aard-static-data",
                                        "Aardwolf Static Data Pass");
