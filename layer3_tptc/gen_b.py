#!/usr/bin/env python3
"""Generate Pass + CodeGen headers."""
import os
BASE = r"d:\Programming\1PRODUCTION\Open Source\tpt-gpu\layer3_tptc"
def w(p, c):
    full = os.path.join(BASE, p)
    os.makedirs(os.path.dirname(full), exist_ok=True)
    with open(full, 'w', encoding='utf-8', newline='\n') as f:
        f.write(c.lstrip('\n'))
    print(f"  {p}")

w("include/tptir/Pass/TPTIRPasses.h", """
#ifndef TPTIR_PASS_TPTIRPASSES_H
#define TPTIR_PASS_TPTIRPASSES_H
#include "../Dialect/TPTIRDialect.h"
#include "../Dialect/TPTIROps.h"
#include <unordered_set>
namespace tptir {
class Pass {
public:
  explicit Pass(const std::string& n) : name_(n) {}
  virtual ~Pass() = default;
  const std::string& name() const { return name_; }
  virtual bool run(Region* region) = 0;
  virtual bool runOnBlock(Block* block);
  virtual std::string statistics() const { return ""; }
private:
  std::string name_;
};
class PassPipeline {
public:
  PassPipeline() = default;
  ~PassPipeline();
  void addPass(Pass* pass);
  size_t run(Region* region);
  const std::vector<Pass*>& passes() const { return passes_; }
  size_t numPasses() const { return passes_.size(); }
  void clear();
private:
  std::vector<Pass*> passes_;
};
class CanonicalizePass : public Pass {
public:
  CanonicalizePass() : Pass("canonicalize") {}
  bool run(Region* region) override;
  std::string statistics() const override;
private:
  size_t foldCount_ = 0;
  bool foldIdentityOps(Block* block);
  bool removeUnreachableBlocks(Region* region);
};
class DeadCodeEliminationPass : public Pass {
public:
  DeadCodeEliminationPass() : Pass("dce") {}
  bool run(Region* region) override;
  std::string statistics() const override;
private:
  size_t removedOps_ = 0;
  void markUsedOps(Block*, std::unordered_set<Operation*>&);
  bool sweepUnusedOps(Block*, const std::unordered_set<Operation*>&);
};
class ConstantFolderPass : public Pass {
public:
  ConstantFolderPass() : Pass("const-fold") {}
  bool run(Region* region) override;
  std::string statistics() const override;
private:
  size_t foldedOps_ = 0;
  Value* tryFold(Operation* op);
  bool foldBlock(Block* block);
};
class VectorizePass : public Pass {
public:
  VectorizePass() : Pass("vectorize") {}
  bool run(Region* region) override;
  std::string statistics() const override;
private:
  size_t vectorizedOps_ = 0;
  bool vectorizeLoops(Region*);
};
class TensorLoweringPass : public Pass {
public:
  TensorLoweringPass() : Pass("tensor-lower") {}
  bool run(Region* region) override;
  std::string statistics() const override;
private:
  size_t loweredOps_ = 0;
  bool lowerTensorOps(Block*);
  bool lowerMMA(Operation*, Block*, size_t);
};
PassPipeline* createDefaultPassPipeline();
PassPipeline* createMinimalPassPipeline();
std::vector<std::string> getAvailablePassNames();
}
#endif
""")

w("include/tptir/CodeGen/TPTCodeGen.h", """
#ifndef TPTIR_CODEGEN_TPTCODEGEN_H
#define TPTIR_CODEGEN_TPTCODEGEN_H
#include "../Dialect/TPTIRDialect.h"
#include "../Dialect/TPTIROps.h"
#include <sstream>
namespace tptir {
enum class CodeGenTarget : uint8_t { TPTISA, LLVMIR, TPTIRText };
struct CodeGenOptions {
  CodeGenTarget target{CodeGenTarget::TPTISA};
  bool optimize{true}; bool emitComments{true};
  std::string entryFunction{"main"};
};
class TPTCodeGen {
public:
  explicit TPTCodeGen(const CodeGenOptions& opts = CodeGenOptions());
  std::string generate(Region* region);
  std::string generateFromBlocks(const std::vector<Block*>& blocks);
  const std::string& lastError() const { return lastError_; }
private:
  std::string emitTPTISA(Region*);
  std::string emitLLVMIR(Region*);
  std::string emitTPTIRText(Region*);
  std::string emitOperation(const Operation*, std::stringstream&);
  std::string valueName(const Value*) const;
  std::string typeName(const Type*) const;
  CodeGenOptions options_;
  std::string lastError_, stats_;
  uint64_t nextLabelId_{0};
  std::unordered_map<const Value*, std::string> valueNames_;
  std::unordered_map<const Block*, std::string> blockLabels_;
};
}
#endif
""")

print("Pass+CodeGen headers done!")
