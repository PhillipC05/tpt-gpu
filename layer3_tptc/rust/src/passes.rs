use crate::ir::Region;
pub trait Pass{fn name(&self)->&str;fn run(&self,region:&Region)->usize;}
pub struct CanonicalizePass;
impl Pass for CanonicalizePass{fn name(&self)->&str{"canonicalize"}fn run(&self,_:&Region)->usize{0}}
pub struct DeadCodeEliminationPass;
impl Pass for DeadCodeEliminationPass{fn name(&self)->&str{"dce"}fn run(&self,_:&Region)->usize{0}}
pub struct PassPipeline{passes:Vec<Box<dyn Pass>>}
impl PassPipeline{
 pub fn new()->Self{PassPipeline{passes:vec![]}}
 pub fn add(&mut self,pass:Box<dyn Pass>){self.passes.push(pass);}
 pub fn run(&self,r:&Region)->usize{let mut t=0;for p in&self.passes{t+=p.run(r);}t}
}
pub fn default_pipeline()->PassPipeline{
 let mut p=PassPipeline::new();
 p.add(Box::new(CanonicalizePass));p.add(Box::new(DeadCodeEliminationPass));p
}
