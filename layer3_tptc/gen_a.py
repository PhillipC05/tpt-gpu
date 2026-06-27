#!/usr/bin/env python3
"""Generate Parser and IR Builder headers."""
import os
BASE = r"d:\Programming\1PRODUCTION\Open Source\tpt-gpu\layer3_tptc"
def w(p, c):
    full = os.path.join(BASE, p)
    os.makedirs(os.path.dirname(full), exist_ok=True)
    with open(full, 'w', encoding='utf-8', newline='\n') as f:
        f.write(c.lstrip('\n'))
    print(f"  {p}")

w("include/tptir/Parser/TPTAsmParser.h", """
#ifndef TPTIR_PARSER_TPTASMPARSER_H
#define TPTIR_PARSER_TPTASMPARSER_H
#include "../Dialect/TPTIRDialect.h"
#include "../Dialect/TPTIROps.h"
namespace tptir {
enum class TokenKind : uint8_t {
  Identifier, Integer, Float, String,
  LParen, RParen, LBrace, RBrace, LBracket, RBracket,
  Less, Greater, Comma, Colon, Arrow, Equals, At,
  Plus, Minus, Star, Slash, Percent,
  KwModule, KwFunc, KwMemref, KwTensor, KwVector,
  KwGlobal, KwShared, KwLocal, KwConstant, Eof, Unknown
};
struct Token { TokenKind kind; std::string text; size_t line; size_t column; };
class Lexer {
public:
  explicit Lexer(const std::string& input);
  Token nextToken();
  Token peekToken();
  const std::vector<Token>& tokens() const { return tokens_; }
  void tokenizeAll();
private:
  void skipWhitespace(); void skipComment();
  Token scanIdentifier(); Token scanNumber();
  char peek() const; char advance(); bool isAtEnd() const;
  std::string input_;
  size_t pos_{0}, line_{1}, column_{1}, tokenPos_{0};
  std::vector<Token> tokens_;
};
class TPTAsmParser {
public:
  explicit TPTAsmParser(const std::string& input);
  Region* parseFunction();
  std::vector<Block*> parseModule();
  const std::string& lastError() const { return lastError_; }
  const std::string& functionName() const { return funcName_; }
private:
  Token consume(TokenKind expected, const std::string& errMsg);
  Token peek() const; Token advance(); bool match(TokenKind kind);
  Type* parseType(); std::vector<int64_t> parseShape();
  AddressSpace parseAddressSpace();
  Block* parseBlock(); Operation* parseOperation();
  Value* parseValue(); std::string parseIdentifier();
  Lexer lexer_; std::vector<Token> tokens_; size_t pos_{0};
  std::string lastError_, funcName_;
  std::vector<std::pair<std::string, Type*>> funcArgs_;
  uint64_t nextValueId_{0};
  std::unordered_map<std::string, Value*> valueMap_;
  std::unordered_map<std::string, Block*> blockMap_;
};
}
#endif
""")

w("include/tptir/IR/TPTIRBuilder.h", """
#ifndef TPTIR_IR_TPTIRBUILDER_H
#define TPTIR_IR_TPTIRBUILDER_H
#include "../Dialect/TPTIRDialect.h"
#include "../Dialect/TPTIROps.h"
#include <stack>
namespace tptir {
class IRBuilder {
public:
  IRBuilder();
  void setInsertionPoint(Block* block);
  Block* currentBlock() const { return currentBlock_; }
  Region* currentRegion() const { return currentRegion_; }
  void setCurrentRegion(Region* region) { currentRegion_ = region; }
  Value* createAddi(Value* lhs, Value* rhs);
  Value* createSubi(Value* lhs, Value* rhs);
  Value* createMuli(Value* lhs, Value* rhs);
  Value* createAddf(Value* lhs, Value* rhs);
  Value* createSubf(Value* lhs, Value* rhs);
  Value* createMulf(Value* lhs, Value* rhs);
  Value* createFMA(Value* a, Value* b, Value* c);
  Value* createAnd(Value* lhs, Value* rhs);
  Value* createOr(Value* lhs, Value* rhs);
  Value* createXor(Value* lhs, Value* rhs);
  Value* createCmpEQ(Value* lhs, Value* rhs);
  Value* createCmpLT(Value* lhs, Value* rhs);
  Value* createConstantI32(int32_t value);
  Value* createConstantF32(float value);
  Value* createLoad(Value* memref, Type* resultType);
  void createStore(Value* value, Value* memref);
  Value* createTensorLoad(Value* memref, Type* tensorType);
  void createTensorStore(Value* tensor, Value* memref);
  Value* createMMA(Value* a, Value* b, Value* c, Type* resultType);
  Value* createVectorLoad(Value* memref, Value* index, Type* vecType);
  void createVectorStore(Value* vector, Value* memref, Value* index);
  Value* createVectorAdd(Value* lhs, Value* rhs, Type* vecType);
  void createBranch(Block* target);
  void createCondBranch(Value* condition, Block* trueBlock, Block* falseBlock);
  void createReturn(const std::vector<Value*>& values = {});
  Block* createBlock(const std::string& label = "");
  void pushBlock(Block* block);
  void popBlock();
  Value* makeValue(Type* type);
private:
  Block* currentBlock_{nullptr};
  Region* currentRegion_{nullptr};
  uint64_t nextValueId_{0};
  std::stack<Block*> blockStack_;
};
}
#endif
""")

print("Parser+Builder headers done!")
