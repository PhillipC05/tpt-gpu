#!/usr/bin/env python3
import os
BASE = r"d:\Programming\1PRODUCTION\Open Source\tpt-gpu\layer3_tptc"
def w(p, c):
    full = os.path.join(BASE, p)
    os.makedirs(os.path.dirname(full), exist_ok=True)
    with open(full, 'w', encoding='utf-8', newline='\n') as f:
        f.write(c.lstrip('\n'))
    print(f"  {p}")
w("lib/Pass/TPTIRPasses.cpp", """
#include "../../include/tptir/Pass/TPTIRPasses.h"
namespace tptir {
bool Pass::runOnBlock(Block*){return false;}
PassPipeline::~PassPipeline(){for(auto*p:passes_)delete p;}
void PassPipeline::addPass(Pass* p){passes_.push_back(p);}
size_t PassPipeline::run(Region* r){size_t t=0;for(auto*p:passes_){if(p->run(r))t++;}return t;}
void PassPipeline::clear(){passes_.clear();}
bool CanonicalizePass::run(Region* r){foldCount_=0;for(auto*b:r->blocks()){if(foldIdentityOps(b))foldCount_++;}removeUnreachableBlocks(r);return foldCount_>0;}
bool CanonicalizePass::foldIdentityOps(Block*){return false;}
bool CanonicalizePass::removeUnreachableBlocks(Region*){return false;}
std::string CanonicalizePass::statistics()const{return"canonicalize: folded "+std::to_string(foldCount_);}
bool DeadCodeEliminationPass::run(Region* r){removedOps_=0;for(auto*b:r->blocks()){std::unordered_set<Operation*>u;markUsedOps(b,u);if(sweepUnusedOps(b,u))removedOps_++;}return removedOps_>0;}
void DeadCodeEliminationPass::markUsedOps(Block*b,std::unordered_set<Operation*>&u){for(auto*op:b->operations()){if(op->category()==OpCategory::ControlFlow||op->category()==OpCategory::Special)u.insert(op);if(op->result()&&op->result()->isValid())u.insert(op);}}
bool DeadCodeEliminationPass::sweepUnusedOps(Block*,const std::unordered_set<Operation*>&){return false;}
std::string DeadCodeEliminationPass::statistics()const{return"dce: removed "+std::to_string(removedOps_);}
bool ConstantFolderPass::run(Region* r){foldedOps_=0;for(auto*b:r->blocks())foldBlock(b);return foldedOps_>0;}
bool ConstantFolderPass::foldBlock(Block*){return false;}
Value* ConstantFolderPass::tryFold(Operation*){return nullptr;}
std::string ConstantFolderPass::statistics()const{return"const-fold: "+std::to_string(foldedOps_);}
bool VectorizePass::run(Region* r){vectorizedOps_=0;vectorizeLoops(r);return vectorizedOps_>0;}
bool VectorizePass::vectorizeLoops(Region*){return false;}
std::string VectorizePass::statistics()const{return"vectorize: "+std::to_string(vectorizedOps_);}
bool TensorLoweringPass::run(Region* r){loweredOps_=0;for(auto*b:r->blocks())lowerTensorOps(b);return loweredOps_>0;}
bool TensorLoweringPass::lowerTensorOps(Block* b){bool c=false;for(size_t i=0;i<b->operations().size();i++){if(b->getOperation(i)->name()=="tptir.mma"){if(lowerMMA(b->getOperation(i),b,i)){c=true;loweredOps_++;}}}return c;}
bool TensorLoweringPass::lowerMMA(Operation*,Block*,size_t){return false;}
std::string TensorLoweringPass::statistics()const{return"tensor-lower: "+std::to_string(loweredOps_);}
PassPipeline* createDefaultPassPipeline(){auto*p=new PassPipeline();p->addPass(new CanonicalizePass());p->addPass(new DeadCodeEliminationPass());p->addPass(new ConstantFolderPass());p->addPass(new VectorizePass());p->addPass(new TensorLoweringPass());return p;}
PassPipeline* createMinimalPassPipeline(){auto*p=new PassPipeline();p->addPass(new CanonicalizePass());p->addPass(new DeadCodeEliminationPass());return p;}
std::vector<std::string> getAvailablePassNames(){return{"canonicalize","dce","const-fold","vectorize","tensor-lower"};}
}
""")
w("lib/CodeGen/TPTCodeGen.cpp", """
#include "../../include/tptir/CodeGen/TPTCodeGen.h"
namespace tptir {
TPTCodeGen::TPTCodeGen(const CodeGenOptions& o):options_(o),nextLabelId_(0){}
std::string TPTCodeGen::generate(Region* r){switch(options_.target){case CodeGenTarget::TPTISA:return emitTPTISA(r);case CodeGenTarget::LLVMIR:return emitLLVMIR(r);case CodeGenTarget::TPTIRText:return emitTPTIRText(r);}return"";}
std::string TPTCodeGen::generateFromBlocks(const std::vector<Block*>& b){Region r;for(auto*b2:b)r.addBlock(b2);return generate(&r);}
std::string TPTCodeGen::emitTPTISA(Region* r){std::stringstream ss;ss<<"; TPT ISA\\n";for(auto*b:r->blocks())for(auto*op:b->operations())ss<<emitOperation(op,ss)<<"\\n";return ss.str();}
std::string TPTCodeGen::emitLLVMIR(Region* r){std::stringstream ss;ss<<"; LLVM IR\\ndefine void @kernel(){\\n";for(auto*b:r->blocks())for(auto*op:b->operations())ss<<"  "<<emitOperation(op,ss)<<"\\n";ss<<"}\\n";return ss.str();}
std::string TPTCodeGen::emitTPTIRText(Region* r){return r->toString();}
std::string TPTCodeGen::emitOperation(const Operation* op,std::stringstream&){return op->toString();}
std::string TPTCodeGen::valueName(const Value* v)const{auto it=valueNames_.find(v);return it!=valueNames_.end()?it->second:v?"%v"+std::to_string(v->id()):"%undef";}
std::string TPTCodeGen::typeName(const Type* t)const{return t?t->toString():"none";}
}
""")
print("Pass + CodeGen .cpp done!")
