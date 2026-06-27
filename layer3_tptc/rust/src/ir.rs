use std::fmt;
#[derive(Debug,Clone,Copy,PartialEq)]
pub enum AddressSpace{Global,Shared,Local,Constant,Generic}
impl fmt::Display for AddressSpace{
 fn fmt(&self,f:&mut fmt::Formatter)->fmt::Result{match self{Self::Global=>write!(f,"global"),Self::Shared=>write!(f,"shared"),Self::Local=>write!(f,"local"),Self::Constant=>write!(f,"constant"),Self::Generic=>write!(f,"generic")}}
}
#[derive(Debug,Clone,PartialEq)]
pub enum TypeKind{I1,I8,I16,I32,I64,F16,BF16,F32,F64,Index,Tensor(Vec<i64>,Box<Type>,AddressSpace),Vector(u32,Box<Type>),MemRef(Vec<i64>,Box<Type>,AddressSpace),Function(Vec<Type>,Vec<Type>),None}
#[derive(Debug,Clone,PartialEq)]
pub struct Type{pub kind:TypeKind}
impl Type{
 pub fn primitive(name:&str)->Self{let kind=match name{"i1"=>TypeKind::I1,"i8"=>TypeKind::I8,"i16"=>TypeKind::I16,"i32"=>TypeKind::I32,"i64"=>TypeKind::I64,"f16"=>TypeKind::F16,"bf16"=>TypeKind::BF16,"f32"=>TypeKind::F32,"f64"=>TypeKind::F64,"index"=>TypeKind::Index,_=>TypeKind::None};Type{kind}}
 pub fn tensor(shape:Vec<i64>,el:Type,as:AddressSpace)->Self{Type{kind:TypeKind::Tensor(shape,Box::new(el),as)}}
 pub fn vector(lanes:u32,el:Type)->Self{Type{kind:TypeKind::Vector(lanes,Box::new(el))}}
 pub fn memref(shape:Vec<i64>,el:Type,as:AddressSpace)->Self{Type{kind:TypeKind::MemRef(shape,Box::new(el),as)}}
}
impl fmt::Display for Type{
 fn fmt(&self,f:&mut fmt::Formatter)->fmt::Result{
  match&self.kind{
   TypeKind::I1=>write!(f,"i1"),TypeKind::I8=>write!(f,"i8"),TypeKind::I16=>write!(f,"i16"),TypeKind::I32=>write!(f,"i32"),TypeKind::I64=>write!(f,"i64"),TypeKind::F16=>write!(f,"f16"),TypeKind::BF16=>write!(f,"bf16"),TypeKind::F32=>write!(f,"f32"),TypeKind::F64=>write!(f,"f64"),TypeKind::Index=>write!(f,"index"),
   TypeKind::Tensor(s,e,as)=>{let v:Vec<String>=s.iter().map(|d|if*d<0{"*".into()}else{d.to_string()}).collect();write!(f,"tensor<{}x{}",v.join("x"),e)?;if*as!=AddressSpace::Global{write!(f,", {}",as)?;}write!(f,">")}
   TypeKind::Vector(l,e)=>write!(f,"vector<{}x{}>",l,e),
   TypeKind::MemRef(s,e,as)=>{let v:Vec<String>=s.iter().map(|d|if*d<0{"*".into()}else{d.to_string()}).collect();write!(f,"memref<{}x{}",v.join("x"),e)?;if*as!=AddressSpace::Global{write!(f,", {}",as)?;}write!(f,">")}
   TypeKind::Function(i,o)=>{let a:Vec<String>=i.iter().map(|t|t.to_string()).collect();let b:Vec<String>=o.iter().map(|t|t.to_string()).collect();write!(f,"({}) -> ({})",a.join(", "),b.join(", "))}
   TypeKind::None=>write!(f,"none"),
}}}
#[derive(Debug,Clone)]
pub enum OpKind{Addi,Subi,Muli,Addf,Subf,Mulf,And,Or,Xor,CmpEq,CmpLt,Load,Store,Branch,Return,Constant(String),Custom(String)}
#[derive(Debug,Clone)]
pub struct Value{pub id:u64,pub typ:Type}
impl Value{pub fn new(id:u64,typ:Type)->Self{Value{id,typ}}}
#[derive(Debug,Clone)]
pub struct Operation{pub kind:OpKind,pub operands:Vec<Value>,pub result_type:Option<Type>,pub result_id:Option<u64>}
impl Operation{
 pub fn new(kind:OpKind)->Self{Operation{kind,operands:vec![],result_type:None,result_id:None}}
 pub fn display(&self)->String{
  match&self.kind{
   OpKind::Addi=>"addi",OpKind::Subi=>"subi",OpKind::Muli=>"muli",
   OpKind::Addf=>"addf",OpKind::Subf=>"subf",OpKind::Mulf=>"mulf",
   OpKind::And=>"andi",OpKind::Or=>"ori",OpKind::Xor=>"xori",
   OpKind::CmpEq=>"cmpeq",OpKind::CmpLt=>"cmplt",
   OpKind::Load=>"load",OpKind::Store=>"store",
   OpKind::Branch=>"br",OpKind::Return=>"return",
   OpKind::Constant(v)=>return format!("constant {}",v),
   _=>"custom",
  }.to_string()
 }
}
#[derive(Debug,Clone)]
pub struct Block{pub label:String,pub operations:Vec<Operation>,pub arguments:Vec<Value>}
impl Block{pub fn new(label:&str)->Self{Block{label:label.to_string(),operations:vec![],arguments:vec![]}}}
#[derive(Debug,Clone)]
pub struct Region{pub blocks:Vec<Block>}
impl Region{pub fn new()->Self{Region{blocks:vec![]}}}
impl fmt::Display for Region{
 fn fmt(&self,f:&mut fmt::Formatter)->fmt::Result{
  for b in &self.blocks{writeln!(f,"^{}: ",b.label)?;for op in &b.operations{writeln!(f,"  {}",op.display())?;}}
  Ok(())
 }
}
pub fn parse_assembly(source:&str)->Result<Region,String>{
 let mut region=Region::new();
 let mut block=Block::new("entry");
 for line in source.lines(){
  let line=line.trim();
  if line.is_empty()||line.starts_with(';')||line.starts_with('#'){continue;}
  if line.starts_with('^'){if!block.operations.is_empty(){region.blocks.push(std::mem::replace(&mut block,Block::new(&line[1..])));}}
 }
 if!block.operations.is_empty()||region.blocks.is_empty(){region.blocks.push(block);}
 Ok(region)
}
