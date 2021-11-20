use std::borrow::Cow;
use wasm_encoder::BlockType;
use wasm_encoder::CodeSection;
use wasm_encoder::Function;
use wasm_encoder::Instruction;
use wasm_encoder::MemArg;
use wasm_encoder::Module;
use wasm_encoder::ValType;
use wasmparser::Chunk;
use wasmparser::CodeSectionReader;
use wasmparser::MemoryImmediate;
use wasmparser::Operator;
use wasmparser::Parser;
use wasmparser::Payload;
use wasmparser::Result;
use wasmparser::SectionReader;
use wasmparser::Type;
use wasmparser::TypeOrFuncType;

/// WebAssembly metering
pub struct Metering {
  gas: u32,
}

pub enum Error {
  InvalidModule,
}

impl Metering {
  pub fn new() -> Self {
    Self { gas: 0 }
  }

  pub fn inject(&mut self, module: &[u8]) -> Result<()> {
    let mut parser = Parser::new(0);
    loop {
      let (payload, consumed) = match parser.parse(module, true)? {
        Chunk::NeedMoreData(hint) => unreachable!(),
        Chunk::Parsed { consumed, payload } => (payload, consumed),
      };

      match payload {
        Payload::StartSection { func: _, range: _ } => {
          // TODO: This is not a smart contract
          // return Err(Error::InvalidModule);
        }
        Payload::CodeSectionStart {
          count: _,
          range,
          size: _,
        } => {
          let section = &module[range.start..range.end];
          parser.skip_section();
          let mut reader = CodeSectionReader::new(section, 0)?;

          let mut section = CodeSection::new();
          for body in reader {
            let body = body?;
            // Preserve the locals.
            let locals = body.get_locals_reader()?;
            let locals =
              locals.into_iter().collect::<Result<Vec<(u32, Type)>>>()?;
            let locals: Vec<(u32, ValType)> =
              locals.into_iter().map(|(i, t)| (i, map_type(t))).collect();
            let mut func = Function::new(locals);

            let mut operators = body.get_operators_reader()?;
            let operators =
              operators.into_iter().collect::<Result<Vec<Operator>>>()?;

            for op in operators {
              let instruction = match op {
                Operator::Unreachable => Instruction::Unreachable,
                Operator::Nop => Instruction::Nop,
                Operator::Block { ty, .. } => match ty {
                  TypeOrFuncType::Type(t) => {
                    Instruction::Block(BlockType::Result(map_type(t)))
                  }
                  TypeOrFuncType::FuncType(idx) => {
                    Instruction::Block(BlockType::FunctionType(idx))
                  }
                },
                Operator::Loop { ty, .. } => match ty {
                  TypeOrFuncType::Type(t) => {
                    Instruction::Block(BlockType::Result(map_type(t)))
                  }
                  TypeOrFuncType::FuncType(idx) => {
                    Instruction::Loop(BlockType::FunctionType(idx))
                  }
                },
                Operator::If { ty, .. } => match ty {
                  TypeOrFuncType::Type(t) => {
                    Instruction::If(BlockType::Result(map_type(t)))
                  }
                  TypeOrFuncType::FuncType(idx) => {
                    Instruction::If(BlockType::FunctionType(idx))
                  }
                },
                Operator::Else => Instruction::Else,
                Operator::Try { ty, .. } => match ty {
                  TypeOrFuncType::Type(t) => {
                    Instruction::Try(BlockType::Result(map_type(t)))
                  }
                  TypeOrFuncType::FuncType(idx) => {
                    Instruction::Try(BlockType::FunctionType(idx))
                  }
                },
                Operator::Catch { index } => Instruction::Catch(index),
                Operator::Throw { index } => Instruction::Throw(index),
                Operator::Rethrow { relative_depth } => {
                  Instruction::Rethrow(relative_depth)
                }
                Operator::End => Instruction::End,
                Operator::Br { relative_depth } => {
                  Instruction::Br(relative_depth)
                }
                Operator::BrIf { relative_depth } => {
                  Instruction::BrIf(relative_depth)
                }
                Operator::BrTable { table } => Instruction::BrTable(
                  table.targets().collect::<Result<Cow<'_, [u32]>>>()?,
                  table.default(),
                ),
                Operator::Return => Instruction::Return,
                Operator::Call { function_index } => {
                  Instruction::Call(function_index)
                }
                Operator::CallIndirect {
                  index: ty,
                  table_index: table,
                } => Instruction::CallIndirect { ty, table },
                // Tail-call proposal
                // https://github.com/WebAssembly/tail-call/blob/master/proposals/tail-call/Overview.md
                //
                // Operator::ReturnCall => Instruction::ReturnCall,
                // Operator::ReturnCallIndirect => Instruction::ReturnCallIndirect,
                Operator::Delegate { relative_depth } => {
                  Instruction::Delegate(relative_depth)
                }
                Operator::CatchAll => Instruction::CatchAll,
                Operator::Drop => Instruction::Drop,
                Operator::Select => Instruction::Select,
                Operator::TypedSelect { ty } => {
                  Instruction::TypedSelect(map_type(ty))
                }
                Operator::LocalGet { local_index } => {
                  Instruction::LocalGet(local_index)
                }
                Operator::LocalSet { local_index } => {
                  Instruction::LocalSet(local_index)
                }
                Operator::LocalTee { local_index } => {
                  Instruction::LocalTee(local_index)
                }
                Operator::GlobalGet { global_index } => {
                  Instruction::GlobalGet(global_index)
                }
                Operator::GlobalSet { global_index } => {
                  Instruction::GlobalSet(global_index)
                }
                Operator::I32Load { memarg } => {
                  Instruction::I32Load(map_memarg(&memarg))
                }
                _ => unimplemented!(),
              };
            }
          }
        }
        Payload::End => break Ok(()),
        _ => {}
      }
    }
  }
}

fn map_type(t: Type) -> ValType {
  match t {
    Type::I32 => ValType::I32,
    Type::I64 => ValType::I64,
    Type::F32 => ValType::F32,
    Type::F64 => ValType::F64,
    Type::V128 => ValType::V128,
    Type::ExternRef => ValType::ExternRef,
    Type::FuncRef => ValType::FuncRef,
    // TODO: Figure this out.
    _ => panic!("unsupported type"),
  }
}

fn map_memarg(memarg: &MemoryImmediate) -> MemArg {
  MemArg {
    offset: memarg.offset,
    align: memarg.align as u32,
    memory_index: memarg.memory,
  }
}
