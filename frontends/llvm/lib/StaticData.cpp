#include "StaticData.h"

#include "llvm/IR/Module.h"
#include "llvm/Support/raw_ostream.h"

#include "StatementDetection.h"
#include "StatementRepository.h"

using namespace aardwolf;

std::string getFilepath(llvm::DebugLoc Loc) {
    if (Loc->getScope()->getDirectory() == "") {
        return Loc->getScope()->getFilename().str();
    } else {
        return (Loc->getScope()->getDirectory() + "/" + Loc->getScope()->getFilename()).str();
    }
}

void exportStatement(StatementRepository& Repo, llvm::raw_ostream& Stream, Statement& Stmt, std::vector<Statement*>& Successors) {
    // Statement id.
    uint64_t Id = Repo.getStatementId(Stmt);
    Stream << "#" << Id;

    // Successors
    if (Successors.size() > 0) {
        Stream << " -> ";

        auto It = Successors.begin();
        Id = Repo.getStatementId(**It);
        Stream << "#" << Id;

        while (++It != Successors.end()) {
            Id = Repo.getStatementId(**It);
            Stream << ", #" << Id;
        }
    }

    // Statement ids / values ids delimiter.
    Stream << "  ::  ";

    // Input values.
    if (Stmt.In.size() > 0) {
        auto It = Stmt.In.begin();
        Id = Repo.getValueId(*It);
        Stream << "%" << Id;
        
        while (++It != Stmt.In.end()) {
            Id = Repo.getValueId(*It);
            Stream << ", %" << Id;
        }
    }

    // Output / inputs delimiter.
    Stream << " ; ";

    // Output value.
    if (Stmt.Out != nullptr) {
        Id = Repo.getValueId(Stmt.Out);
        Stream << "%" << Id;
    }

    // Location.
    Id = Repo.getFileId(getFilepath(Stmt.Loc));
    Stream << " [@" << Id << ", " << Stmt.Loc.getLine() << ", " << Stmt.Loc.getCol() << "]\n";
}

void exportMetadata(StatementRepository& Repo, llvm::raw_ostream& Stream) {
    for (auto F : Repo.FilesIdMap) {
        Stream << "@" << F.second << " = " << F.first << "\n";
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

    std::vector<Statement*> Outgoing;
    auto Repo = getAnalysis<StatementDetection>().Repo;

    for (auto &F : M) {
        if (F.isDeclaration()) {
            continue;
        }

        Stream << F.getName() << "\n\n";

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

        Stream << "\n";
    }

    Stream << "\n";
    exportMetadata(Repo, Stream);

    return false;
}

void StaticData::getAnalysisUsage(llvm::AnalysisUsage &AU) const {
    AU.setPreservesAll();
    AU.addRequired<StatementDetection>();
}

char StaticData::ID = 0;
static llvm::RegisterPass<StaticData> X("aard-static-data", "Aardwolf Static Data Pass");
