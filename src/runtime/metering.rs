use std::borrow::Cow;
use std::mem::transmute;
use wasm_encoder::BlockType;
use wasm_encoder::CodeSection;
use wasm_encoder::ElementSection;
use wasm_encoder::EntityType;
use wasm_encoder::Function;
use wasm_encoder::ImportSection;
use wasm_encoder::Instruction;
use wasm_encoder::MemArg;
use wasm_encoder::Module;
use wasm_encoder::RawSection;
use wasm_encoder::SectionId;
use wasm_encoder::StartSection;
use wasm_encoder::TypeSection;
use wasm_encoder::ValType;
use wasmparser::Chunk;
use wasmparser::CodeSectionReader;
use wasmparser::ImportSectionEntryType;
use wasmparser::MemoryImmediate;
use wasmparser::Operator;
use wasmparser::Parser;
use wasmparser::Payload;
use wasmparser::Result;
use wasmparser::SectionReader;
use wasmparser::Type;
use wasmparser::TypeDef;
use wasmparser::TypeOrFuncType;

/// 3EM's WebAssembly metering module.
pub struct Metering(
  // Cost function
  Box<dyn Fn(&Instruction) -> i32>,
);

impl Metering {
  pub fn new<T>(cost_fn: T) -> Self
  where
    T: Fn(&Instruction) -> i32 + 'static,
  {
    Self(Box::new(cost_fn))
  }

  pub fn inject(&self, input: &[u8]) -> Result<Module> {
    let mut source = input;
    let mut parser = Parser::new(0);
    let mut module = Module::new();
    let mut consume_gas_index = -1;
    let mut func_idx: i32 = -1;

    // Temporary store for payloads.
    // We'd want to wait until the imports section
    // is processed i.e. func_idx >= 0
    let mut pending_payloads = vec![];

    loop {
      let (payload, consumed) = match parser.parse(source, true)? {
        Chunk::NeedMoreData(hint) => unreachable!(),
        Chunk::Parsed { consumed, payload } => (payload, consumed),
      };

      match payload {
        Payload::ImportSection(mut reader) => {
          let range = reader.range();

          let mut imports = ImportSection::new();

          for import in reader {
            let import = import?;
            let ty = match import.ty {
              ImportSectionEntryType::Function(ty) => {
                if func_idx == -1 {
                  func_idx = 0
                };
                func_idx += 1;
                EntityType::Function(ty)
              }
              ImportSectionEntryType::Table(wasmparser::TableType {
                element_type,
                initial: minimum,
                maximum,
              }) => EntityType::Table(wasm_encoder::TableType {
                element_type: map_type(element_type),
                minimum,
                maximum,
              }),
              ImportSectionEntryType::Memory(wasmparser::MemoryType {
                memory64,
                shared: _,
                initial: minimum,
                maximum,
              }) => EntityType::Memory(wasm_encoder::MemoryType {
                memory64,
                minimum,
                maximum,
              }),
              ImportSectionEntryType::Tag(wasmparser::TagType {
                type_index: func_type_idx,
              }) => EntityType::Tag(wasm_encoder::TagType {
                kind: wasm_encoder::TagKind::Exception,
                func_type_idx,
              }),
              ImportSectionEntryType::Global(wasmparser::GlobalType {
                mutable,
                content_type,
              }) => EntityType::Global(wasm_encoder::GlobalType {
                mutable,
                val_type: map_type(content_type),
              }),
              ImportSectionEntryType::Module(idx) => EntityType::Module(idx),
              ImportSectionEntryType::Instance(idx) => {
                EntityType::Instance(idx)
              }
            };

            imports.import(import.module, import.field, ty);
          }

          imports.import(
            "3em",
            Some("consumeGas"),
            EntityType::Function(consume_gas_index as u32),
          );

          module.section(&imports);
        }
        Payload::TypeSection(mut reader) => {
          let range = reader.range();

          let mut types = TypeSection::new();

          for ty in reader {
            let ty = ty?;
            match ty {
              TypeDef::Func(func) => {
                let params: Vec<ValType> =
                  func.params.iter().map(|ty| map_type(*ty)).collect();
                let returns: Vec<ValType> =
                  func.returns.iter().map(|ty| map_type(*ty)).collect();

                types.function(params, returns);
              }
              TypeDef::Instance(instance) => {
                let exports: Vec<(&str, EntityType)> = instance
                  .exports
                  .iter()
                  .map(|export| {
                    let ty = match export.ty {
                      ImportSectionEntryType::Function(ty) => {
                        EntityType::Function(ty)
                      }
                      ImportSectionEntryType::Table(
                        wasmparser::TableType {
                          element_type,
                          initial: minimum,
                          maximum,
                        },
                      ) => EntityType::Table(wasm_encoder::TableType {
                        element_type: map_type(element_type),
                        minimum,
                        maximum,
                      }),
                      ImportSectionEntryType::Memory(
                        wasmparser::MemoryType {
                          memory64,
                          shared: _,
                          initial: minimum,
                          maximum,
                        },
                      ) => EntityType::Memory(wasm_encoder::MemoryType {
                        memory64,
                        minimum,
                        maximum,
                      }),
                      ImportSectionEntryType::Tag(wasmparser::TagType {
                        type_index: func_type_idx,
                      }) => EntityType::Tag(wasm_encoder::TagType {
                        kind: wasm_encoder::TagKind::Exception,
                        func_type_idx,
                      }),
                      ImportSectionEntryType::Global(
                        wasmparser::GlobalType {
                          mutable,
                          content_type,
                        },
                      ) => EntityType::Global(wasm_encoder::GlobalType {
                        mutable,
                        val_type: map_type(content_type),
                      }),
                      ImportSectionEntryType::Module(idx) => {
                        EntityType::Module(idx)
                      }
                      ImportSectionEntryType::Instance(idx) => {
                        EntityType::Instance(idx)
                      }
                    };
                    let name = export.name;
                    (name, ty)
                  })
                  .collect();

                types.instance(exports);
              }
              TypeDef::Module(module) => {
                let imports: Vec<(&str, Option<&str>, EntityType)> = module
                  .imports
                  .iter()
                  .map(|import| {
                    let ty = match import.ty {
                      ImportSectionEntryType::Function(ty) => {
                        EntityType::Function(ty)
                      }
                      ImportSectionEntryType::Table(
                        wasmparser::TableType {
                          element_type,
                          initial: minimum,
                          maximum,
                        },
                      ) => EntityType::Table(wasm_encoder::TableType {
                        element_type: map_type(element_type),
                        minimum,
                        maximum,
                      }),
                      ImportSectionEntryType::Memory(
                        wasmparser::MemoryType {
                          memory64,
                          shared: _,
                          initial: minimum,
                          maximum,
                        },
                      ) => EntityType::Memory(wasm_encoder::MemoryType {
                        memory64,
                        minimum,
                        maximum,
                      }),
                      ImportSectionEntryType::Tag(wasmparser::TagType {
                        type_index: func_type_idx,
                      }) => EntityType::Tag(wasm_encoder::TagType {
                        kind: wasm_encoder::TagKind::Exception,
                        func_type_idx,
                      }),
                      ImportSectionEntryType::Global(
                        wasmparser::GlobalType {
                          mutable,
                          content_type,
                        },
                      ) => EntityType::Global(wasm_encoder::GlobalType {
                        mutable,
                        val_type: map_type(content_type),
                      }),
                      ImportSectionEntryType::Module(idx) => {
                        EntityType::Module(idx)
                      }
                      ImportSectionEntryType::Instance(idx) => {
                        EntityType::Instance(idx)
                      }
                    };

                    let module = import.module;
                    let field = import.field;
                    (module, field, ty)
                  })
                  .collect();
                let exports: Vec<(&str, EntityType)> = module
                  .exports
                  .iter()
                  .map(|export| {
                    let ty = match export.ty {
                      ImportSectionEntryType::Function(ty) => {
                        EntityType::Function(ty)
                      }
                      ImportSectionEntryType::Table(
                        wasmparser::TableType {
                          element_type,
                          initial: minimum,
                          maximum,
                        },
                      ) => EntityType::Table(wasm_encoder::TableType {
                        element_type: map_type(element_type),
                        minimum,
                        maximum,
                      }),
                      ImportSectionEntryType::Memory(
                        wasmparser::MemoryType {
                          memory64,
                          shared: _,
                          initial: minimum,
                          maximum,
                        },
                      ) => EntityType::Memory(wasm_encoder::MemoryType {
                        memory64,
                        minimum,
                        maximum,
                      }),
                      ImportSectionEntryType::Tag(wasmparser::TagType {
                        type_index: func_type_idx,
                      }) => EntityType::Tag(wasm_encoder::TagType {
                        kind: wasm_encoder::TagKind::Exception,
                        func_type_idx,
                      }),
                      ImportSectionEntryType::Global(
                        wasmparser::GlobalType {
                          mutable,
                          content_type,
                        },
                      ) => EntityType::Global(wasm_encoder::GlobalType {
                        mutable,
                        val_type: map_type(content_type),
                      }),
                      ImportSectionEntryType::Module(idx) => {
                        EntityType::Module(idx)
                      }
                      ImportSectionEntryType::Instance(idx) => {
                        EntityType::Instance(idx)
                      }
                    };
                    let name = export.name;
                    (name, ty)
                  })
                  .collect();

                types.module(imports, exports);
              }
            }
          }

          types.function([ValType::I32], []);
          consume_gas_index = types.len() as i32 - 1;

          module.section(&types);
        }
        Payload::Version { .. } => {}
        Payload::End => break,
        _ => pending_payloads.push(payload),
      };

      source = &source[consumed..];
    }

    // No types section? Make one :-)
    if consume_gas_index == -1 {
      let mut types = TypeSection::new();
      types.function([ValType::I32], []);
      // This is the only type defined.
      consume_gas_index = 0;

      module.section(&types);
    }

    // There is no import section. Make one.
    if func_idx == -1 {
      let mut imports = ImportSection::new();
      imports.import(
        "3em",
        Some("consumeGas"),
        EntityType::Function(consume_gas_index as u32),
      );
      func_idx = 0;
      module.section(&imports);
    }

    for payload in pending_payloads {
      match payload {
        Payload::StartSection { func, range } => {
          let function_index = if func >= func_idx as u32 {
            func + 1
          } else {
            func
          };

          let start = StartSection { function_index };
          module.section(&start);
        }
        Payload::CodeSectionStart {
          count: _,
          range,
          size: _,
        } => {
          let section = &input[range.start..range.end];

          let mut reader = CodeSectionReader::new(section, 0)?;
          let mut section = CodeSection::new();

          for body in reader {
            let body = body?;
            // Preserve the locals.
            let locals = match body.get_locals_reader() {
              Ok(locals) => {
                locals.into_iter().collect::<Result<Vec<(u32, Type)>>>()?
              }
              Err(_) => vec![],
            };
            let locals: Vec<(u32, ValType)> =
              locals.into_iter().map(|(i, t)| (i, map_type(t))).collect();
            let mut func = Function::new(locals);

            let mut operators = body.get_operators_reader()?;
            let operators =
              operators.into_iter().collect::<Result<Vec<Operator>>>()?;

            for op in operators {
              let instruction = map_operator(op, func_idx as i32)?;

              let cost = self.0(&instruction);
              // There is no such thing as negative cost.
              // If the cost function returns a negative value
              // the gas is skipped.
              if cost >= 0 {
                func.instruction(&Instruction::I32Const(cost));
                func.instruction(&Instruction::Call(func_idx as u32));
              }

              func.instruction(&instruction);
            }
            section.function(&func);
          }
          module.section(&section);

          // parser.skip_section();
          source = &input[range.end..];
          continue;
        }
        Payload::DataCountSection { count: _, range } => {
          module.section(&RawSection {
            id: SectionId::DataCount as u8,
            data: &input[range.start..range.end],
          });
        }
        Payload::FunctionSection(mut reader) => {
          let range = reader.range();
          module.section(&RawSection {
            id: SectionId::Function as u8,
            data: &input[range.start..range.end],
          });
        }
        Payload::TableSection(reader) => {
          let range = reader.range();
          module.section(&RawSection {
            id: SectionId::Table as u8,
            data: &input[range.start..range.end],
          });
        }
        Payload::MemorySection(reader) => {
          let range = reader.range();
          module.section(&RawSection {
            id: SectionId::Memory as u8,
            data: &input[range.start..range.end],
          });
        }
        Payload::GlobalSection(mut reader) => {
          let range = reader.range();
          module.section(&RawSection {
            id: SectionId::Global as u8,
            data: &input[range.start..range.end],
          });
        }
        Payload::ExportSection(mut reader) => {
          let range = reader.range();
          let mut section = wasm_encoder::ExportSection::new();

          for export in reader {
            let export = export?;
            let idx = export.index;
            let field = export.field;

            let export = match export.kind {
              wasmparser::ExternalKind::Function => {
                let idx = if idx >= func_idx as u32 { idx + 1 } else { idx };
                wasm_encoder::Export::Function(idx)
              }
              wasmparser::ExternalKind::Table => {
                wasm_encoder::Export::Table(idx)
              }
              wasmparser::ExternalKind::Memory => {
                wasm_encoder::Export::Memory(idx)
              }
              wasmparser::ExternalKind::Tag => wasm_encoder::Export::Tag(idx),
              wasmparser::ExternalKind::Global => {
                wasm_encoder::Export::Global(idx)
              }
              wasmparser::ExternalKind::Type => {
                unreachable!("No encoder mappings")
              }
              wasmparser::ExternalKind::Module => {
                wasm_encoder::Export::Module(idx)
              }
              wasmparser::ExternalKind::Instance => {
                wasm_encoder::Export::Instance(idx)
              }
            };

            section.export(field, export);
          }
          module.section(&section);
        }
        Payload::ElementSection(reader) => {
          let range = reader.range();
          let mut section = ElementSection::new();
          for element in reader {
            let element = element?;

            let element_type = map_type(element.ty);

            let mut funcs = vec![];
            for item in element.items.get_items_reader().unwrap() {
              match item.unwrap() {
                wasmparser::ElementItem::Func(idx) => {
                  let idx = if idx >= func_idx as u32 { idx + 1 } else { idx };
                  funcs.push(idx);
                }
                wasmparser::ElementItem::Expr(_) => {
                  todo!("Implement Expr Item.")
                }
              }
            }

            let elements = wasm_encoder::Elements::Functions(&funcs);

            match element.kind {
              wasmparser::ElementKind::Passive => {
                section.passive(element_type, elements);
              }
              wasmparser::ElementKind::Active {
                table_index,
                init_expr,
              } => {
                let mut reader = init_expr.get_operators_reader();

                let op = reader.read()?;
                // A "constant-time" instruction
                // (*.const or global.get)
                let offset = map_operator(op, func_idx as i32)?;
                section.active(
                  Some(table_index),
                  &offset,
                  element_type,
                  elements,
                );
              }
              wasmparser::ElementKind::Declared => {
                section.declared(element_type, elements);
              }
            };
          }

          module.section(&section);
        }
        Payload::DataSection(reader) => {
          let range = reader.range();
          module.section(&RawSection {
            id: SectionId::Data as u8,
            data: &input[range.start..range.end],
          });
        }
        Payload::CustomSection {
          name: _,
          data_offset: _,
          data: _,
          range,
        } => {
          module.section(&RawSection {
            id: SectionId::Custom as u8,
            data: &input[range.start..range.end],
          });
        }
        Payload::AliasSection(reader) => {
          let range = reader.range();
          module.section(&RawSection {
            id: SectionId::Alias as u8,
            data: &input[range.start..range.end],
          });
        }
        Payload::UnknownSection {
          id,
          contents: _,
          range,
        } => {
          module.section(&RawSection {
            id,
            data: &input[range.start..range.end],
          });
        }
        Payload::InstanceSection(reader) => {
          let range = reader.range();
          module.section(&RawSection {
            id: SectionId::Instance as u8,
            data: &input[range.start..range.end],
          });
        }
        Payload::TagSection(reader) => {
          let range = reader.range();
          module.section(&RawSection {
            id: SectionId::Tag as u8,
            data: &input[range.start..range.end],
          });
        }
        Payload::CodeSectionEntry(_) => {
          // Already parsed in Payload::CodeSectionStart
          // unreachable!();
        }
        Payload::ModuleSectionStart {
          count: _,
          size: _,
          range,
        } => {
          module.section(&RawSection {
            id: SectionId::Module as u8,
            data: &input[range.start..range.end],
          });
        }
        Payload::ModuleSectionEntry { parser: _, range } => {
          module.section(&RawSection {
            id: SectionId::Module as u8,
            data: &input[range.start..range.end],
          });
        }
        _ => unreachable!(),
      }
    }
    Ok(module)
  }
}

fn map_operator(operator: Operator, gas_idx: i32) -> Result<Instruction> {
  let inst = match operator {
    Operator::Unreachable => Instruction::Unreachable,
    Operator::Nop => Instruction::Nop,
    Operator::Block { ty, .. } => Instruction::Block(map_block_type(ty)),
    Operator::Loop { ty, .. } => Instruction::Loop(map_block_type(ty)),
    Operator::If { ty, .. } => Instruction::If(map_block_type(ty)),
    Operator::Else => Instruction::Else,
    Operator::Try { ty, .. } => Instruction::Try(map_block_type(ty)),
    Operator::Catch { index } => Instruction::Catch(index),
    Operator::Throw { index } => Instruction::Throw(index),
    Operator::Rethrow { relative_depth } => {
      Instruction::Rethrow(relative_depth)
    }
    Operator::End => Instruction::End,
    Operator::Br { relative_depth } => Instruction::Br(relative_depth),
    Operator::BrIf { relative_depth } => Instruction::BrIf(relative_depth),
    Operator::BrTable { table } => Instruction::BrTable(
      table.targets().collect::<Result<Cow<'_, [u32]>>>()?,
      table.default(),
    ),
    Operator::Return => Instruction::Return,
    Operator::Call { function_index } => {
      let function_index = if gas_idx >= 0 && function_index >= gas_idx as u32 {
        function_index + 1
      } else {
        function_index
      };
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
    Operator::TypedSelect { ty } => Instruction::TypedSelect(map_type(ty)),
    Operator::LocalGet { local_index } => Instruction::LocalGet(local_index),
    Operator::LocalSet { local_index } => Instruction::LocalSet(local_index),
    Operator::LocalTee { local_index } => Instruction::LocalTee(local_index),
    Operator::GlobalGet { global_index } => {
      Instruction::GlobalGet(global_index)
    }
    Operator::GlobalSet { global_index } => {
      Instruction::GlobalSet(global_index)
    }
    Operator::I32Load { memarg } => Instruction::I32Load(map_memarg(&memarg)),
    Operator::I64Load { memarg } => Instruction::I64Load(map_memarg(&memarg)),
    Operator::F32Load { memarg } => Instruction::F32Load(map_memarg(&memarg)),
    Operator::F64Load { memarg } => Instruction::F64Load(map_memarg(&memarg)),
    Operator::I32Load8S { memarg } => {
      Instruction::I32Load8_S(map_memarg(&memarg))
    }
    Operator::I32Load8U { memarg } => {
      Instruction::I32Load8_U(map_memarg(&memarg))
    }
    Operator::I32Load16S { memarg } => {
      Instruction::I32Load16_S(map_memarg(&memarg))
    }
    Operator::I32Load16U { memarg } => {
      Instruction::I32Load16_U(map_memarg(&memarg))
    }
    Operator::I64Load8S { memarg } => {
      Instruction::I64Load8_S(map_memarg(&memarg))
    }
    Operator::I64Load8U { memarg } => {
      Instruction::I64Load8_U(map_memarg(&memarg))
    }
    Operator::I64Load16S { memarg } => {
      Instruction::I64Load16_S(map_memarg(&memarg))
    }
    Operator::I64Load16U { memarg } => {
      Instruction::I64Load16_U(map_memarg(&memarg))
    }
    Operator::I64Load32S { memarg } => {
      Instruction::I64Load32_S(map_memarg(&memarg))
    }
    Operator::I64Load32U { memarg } => {
      Instruction::I64Load32_U(map_memarg(&memarg))
    }
    Operator::I32Store { memarg } => Instruction::I32Store(map_memarg(&memarg)),
    Operator::I64Store { memarg } => Instruction::I64Store(map_memarg(&memarg)),
    Operator::F32Store { memarg } => Instruction::F32Store(map_memarg(&memarg)),
    Operator::F64Store { memarg } => Instruction::F64Store(map_memarg(&memarg)),
    Operator::I32Store8 { memarg } => {
      Instruction::I32Store8(map_memarg(&memarg))
    }
    Operator::I32Store16 { memarg } => {
      Instruction::I32Store16(map_memarg(&memarg))
    }
    Operator::I64Store8 { memarg } => {
      Instruction::I64Store8(map_memarg(&memarg))
    }
    Operator::I64Store16 { memarg } => {
      Instruction::I64Store16(map_memarg(&memarg))
    }
    Operator::I64Store32 { memarg } => {
      Instruction::I64Store32(map_memarg(&memarg))
    }
    Operator::MemorySize { mem, mem_byte: _ } => Instruction::MemorySize(mem),
    Operator::MemoryGrow { mem, mem_byte: _ } => Instruction::MemoryGrow(mem),
    Operator::I32Const { value } => Instruction::I32Const(value),
    Operator::I64Const { value } => Instruction::I64Const(value),
    // Floats and Ints have the same endianness on all supported platforms.
    Operator::F32Const { value } => {
      Instruction::F32Const(unsafe { transmute::<u32, f32>(value.bits()) })
    }
    Operator::F64Const { value } => {
      Instruction::F64Const(unsafe { transmute::<u64, f64>(value.bits()) })
    }
    Operator::RefNull { ty } => Instruction::RefNull(map_type(ty)),
    Operator::RefIsNull => Instruction::RefIsNull,
    Operator::RefFunc {
      function_index: index,
    } => Instruction::RefFunc(index),
    Operator::I32Eqz => Instruction::I32Eqz,
    Operator::I32Eq => Instruction::I32Eq,
    Operator::I32Ne => Instruction::I32Neq,
    Operator::I32LtS => Instruction::I32LtS,
    Operator::I32LtU => Instruction::I32LtU,
    Operator::I32GtS => Instruction::I32GtS,
    Operator::I32GtU => Instruction::I32GtU,
    Operator::I32LeS => Instruction::I32LeS,
    Operator::I32LeU => Instruction::I32LeU,
    Operator::I32GeS => Instruction::I32GeS,
    Operator::I32GeU => Instruction::I32GeU,
    Operator::I64Eqz => Instruction::I64Eqz,
    Operator::I64Eq => Instruction::I64Eq,
    Operator::I64Ne => Instruction::I64Neq,
    Operator::I64LtS => Instruction::I64LtS,
    Operator::I64LtU => Instruction::I64LtU,
    Operator::I64GtS => Instruction::I64GtS,
    Operator::I64GtU => Instruction::I64GtU,
    Operator::I64LeS => Instruction::I64LeS,
    Operator::I64LeU => Instruction::I64LeU,
    Operator::I64GeS => Instruction::I64GeS,
    Operator::I64GeU => Instruction::I64GeU,
    Operator::F32Eq => Instruction::F32Eq,
    Operator::F32Ne => Instruction::F32Neq,
    Operator::F32Lt => Instruction::F32Lt,
    Operator::F32Gt => Instruction::F32Gt,
    Operator::F32Le => Instruction::F32Le,
    Operator::F32Ge => Instruction::F32Ge,
    Operator::F64Eq => Instruction::F64Eq,
    Operator::F64Ne => Instruction::F64Neq,
    Operator::F64Lt => Instruction::F64Lt,
    Operator::F64Gt => Instruction::F64Gt,
    Operator::F64Le => Instruction::F64Le,
    Operator::F64Ge => Instruction::F64Ge,
    Operator::I32Clz => Instruction::I32Clz,
    Operator::I32Ctz => Instruction::I32Ctz,
    Operator::I32Popcnt => Instruction::I32Popcnt,
    Operator::I32Add => Instruction::I32Add,
    Operator::I32Sub => Instruction::I32Sub,
    Operator::I32Mul => Instruction::I32Mul,
    Operator::I32DivS => Instruction::I32DivS,
    Operator::I32DivU => Instruction::I32DivU,
    Operator::I32RemS => Instruction::I32RemS,
    Operator::I32RemU => Instruction::I32RemU,
    Operator::I32And => Instruction::I32And,
    Operator::I32Or => Instruction::I32Or,
    Operator::I32Xor => Instruction::I32Xor,
    Operator::I32Shl => Instruction::I32Shl,
    Operator::I32ShrS => Instruction::I32ShrS,
    Operator::I32ShrU => Instruction::I32ShrU,
    Operator::I32Rotl => Instruction::I32Rotl,
    Operator::I32Rotr => Instruction::I32Rotr,
    Operator::I64Clz => Instruction::I64Clz,
    Operator::I64Ctz => Instruction::I64Ctz,
    Operator::I64Popcnt => Instruction::I64Popcnt,
    Operator::I64Add => Instruction::I64Add,
    Operator::I64Sub => Instruction::I64Sub,
    Operator::I64Mul => Instruction::I64Mul,
    Operator::I64DivS => Instruction::I64DivS,
    Operator::I64DivU => Instruction::I64DivU,
    Operator::I64RemS => Instruction::I64RemS,
    Operator::I64RemU => Instruction::I64RemU,
    Operator::I64And => Instruction::I64And,
    Operator::I64Or => Instruction::I64Or,
    Operator::I64Xor => Instruction::I64Xor,
    Operator::I64Shl => Instruction::I64Shl,
    Operator::I64ShrS => Instruction::I64ShrS,
    Operator::I64ShrU => Instruction::I64ShrU,
    Operator::I64Rotl => Instruction::I64Rotl,
    Operator::I64Rotr => Instruction::I64Rotr,
    Operator::F32Abs => Instruction::F32Abs,
    Operator::F32Neg => Instruction::F32Neg,
    Operator::F32Ceil => Instruction::F32Ceil,
    Operator::F32Floor => Instruction::F32Floor,
    Operator::F32Trunc => Instruction::F32Trunc,
    Operator::F32Nearest => Instruction::F32Nearest,
    Operator::F32Sqrt => Instruction::F32Sqrt,
    Operator::F32Add => Instruction::F32Add,
    Operator::F32Sub => Instruction::F32Sub,
    Operator::F32Mul => Instruction::F32Mul,
    Operator::F32Div => Instruction::F32Div,
    Operator::F32Min => Instruction::F32Min,
    Operator::F32Max => Instruction::F32Max,
    Operator::F32Copysign => Instruction::F32Copysign,
    Operator::F64Abs => Instruction::F64Abs,
    Operator::F64Neg => Instruction::F64Neg,
    Operator::F64Ceil => Instruction::F64Ceil,
    Operator::F64Floor => Instruction::F64Floor,
    Operator::F64Trunc => Instruction::F64Trunc,
    Operator::F64Nearest => Instruction::F64Nearest,
    Operator::F64Sqrt => Instruction::F64Sqrt,
    Operator::F64Add => Instruction::F64Add,
    Operator::F64Sub => Instruction::F64Sub,
    Operator::F64Mul => Instruction::F64Mul,
    Operator::F64Div => Instruction::F64Div,
    Operator::F64Min => Instruction::F64Min,
    Operator::F64Max => Instruction::F64Max,
    Operator::F64Copysign => Instruction::F64Copysign,
    Operator::I32WrapI64 => Instruction::I32WrapI64,
    Operator::I32TruncF32S => Instruction::I32TruncF32S,
    Operator::I32TruncF32U => Instruction::I32TruncF32U,
    Operator::I32TruncF64S => Instruction::I32TruncF64S,
    Operator::I32TruncF64U => Instruction::I32TruncF64U,
    Operator::I64ExtendI32S => Instruction::I64ExtendI32S,
    Operator::I64ExtendI32U => Instruction::I64ExtendI32U,
    Operator::I64TruncF32S => Instruction::I64TruncF32S,
    Operator::I64TruncF32U => Instruction::I64TruncF32U,
    Operator::I64TruncF64S => Instruction::I64TruncF64S,
    Operator::I64TruncF64U => Instruction::I64TruncF64U,
    Operator::F32ConvertI32S => Instruction::F32ConvertI32S,
    Operator::F32ConvertI32U => Instruction::F32ConvertI32U,
    Operator::F32ConvertI64S => Instruction::F32ConvertI64S,
    Operator::F32ConvertI64U => Instruction::F32ConvertI64U,
    Operator::F32DemoteF64 => Instruction::F32DemoteF64,
    Operator::F64ConvertI32S => Instruction::F64ConvertI32S,
    Operator::F64ConvertI32U => Instruction::F64ConvertI32U,
    Operator::F64ConvertI64S => Instruction::F64ConvertI64S,
    Operator::F64ConvertI64U => Instruction::F64ConvertI64U,
    Operator::F64PromoteF32 => Instruction::F64PromoteF32,
    Operator::I32ReinterpretF32 => Instruction::I32ReinterpretF32,
    Operator::I64ReinterpretF64 => Instruction::I64ReinterpretF64,
    Operator::F32ReinterpretI32 => Instruction::F32ReinterpretI32,
    Operator::F64ReinterpretI64 => Instruction::F64ReinterpretI64,
    Operator::I32Extend8S => Instruction::I32Extend8S,
    Operator::I32Extend16S => Instruction::I32Extend16S,
    Operator::I64Extend8S => Instruction::I64Extend8S,
    Operator::I64Extend16S => Instruction::I64Extend16S,
    Operator::I64Extend32S => Instruction::I64Extend32S,
    Operator::I32TruncSatF32S => Instruction::I32TruncSatF32S,
    Operator::I32TruncSatF32U => Instruction::I32TruncSatF32U,
    Operator::I32TruncSatF64S => Instruction::I32TruncSatF64S,
    Operator::I32TruncSatF64U => Instruction::I32TruncSatF64U,
    Operator::I64TruncSatF32S => Instruction::I64TruncSatF32S,
    Operator::I64TruncSatF32U => Instruction::I64TruncSatF32U,
    Operator::I64TruncSatF64S => Instruction::I64TruncSatF64S,
    Operator::I64TruncSatF64U => Instruction::I64TruncSatF64U,
    Operator::MemoryInit { mem, segment: data } => {
      Instruction::MemoryInit { mem, data }
    }
    Operator::DataDrop { segment: data } => Instruction::DataDrop(data),
    Operator::MemoryCopy { dst, src } => Instruction::MemoryCopy { dst, src },
    Operator::MemoryFill { mem } => Instruction::MemoryFill(mem),
    Operator::TableInit { table, segment } => {
      Instruction::TableInit { segment, table }
    }
    Operator::ElemDrop { segment } => Instruction::ElemDrop { segment },
    Operator::TableCopy {
      dst_table: dst,
      src_table: src,
    } => Instruction::TableCopy { dst, src },
    Operator::TableFill { table } => Instruction::TableFill { table },
    Operator::TableGet { table } => Instruction::TableGet { table },
    Operator::TableSet { table } => Instruction::TableSet { table },
    Operator::TableGrow { table } => Instruction::TableGrow { table },
    Operator::TableSize { table } => Instruction::TableSize { table },
    // WebAssembly threads proposal.
    // https://github.com/webassembly/threads
    // Operator::MemoryAtomicNotify => {},
    // Operator::MemoryAtomicWait32 => {},
    // Operator::MemoryAtomicWait64 => {},
    // Operator::AtomicFench => {},
    // Operator::I32AtomicLoad => {},
    // Operator::I64AtomicLoad => {},
    // ...
    //
    // SIMD proposal.
    Operator::V128Load { memarg } => Instruction::V128Load {
      memarg: map_memarg(&memarg),
    },
    _ => unimplemented!(),
  };

  Ok(inst)
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
    Type::EmptyBlockType | Type::Func => unreachable!(),
    Type::ExnRef => panic!("unsupported type"),
  }
}

fn map_block_type(ty: TypeOrFuncType) -> BlockType {
  match ty {
    TypeOrFuncType::Type(t) => match t {
      Type::EmptyBlockType => BlockType::Empty,
      Type::Func => unreachable!(),
      _ => BlockType::Result(map_type(t)),
    },
    TypeOrFuncType::FuncType(idx) => BlockType::FunctionType(idx),
  }
}

fn map_memarg(memarg: &MemoryImmediate) -> MemArg {
  MemArg {
    offset: memarg.offset,
    align: memarg.align as u32,
    memory_index: memarg.memory,
  }
}

#[cfg(test)]
mod tests {
  use crate::runtime::metering::Metering;
  use crate::runtime::wasm::WasmRuntime;
  use deno_core::serde_json;
  use deno_core::serde_json::json;
  use deno_core::serde_json::Value;
  use wasm_encoder::Instruction;

  fn test_cost_function(_: &Instruction) -> i32 {
    1
  }

  #[tokio::test]
  async fn test_metering_contracts() {
    let metering = Metering::new(test_cost_function);
    // (expected gas consumption, module bytes)
    let sources: [(usize, &[u8]); 2] = [
      (26300, include_bytes!("./testdata/01_wasm/01_wasm.wasm")),
      (38888, include_bytes!("./testdata/02_wasm/02_wasm.wasm")),
    ];

    for source in sources {
      let module = metering.inject(source.1).unwrap();

      let mut rt = WasmRuntime::new(&module.finish(), Default::default()).await.unwrap();

      let mut prev_state = json!({
        "counter": 0,
      });
      let mut prev_state_bytes = serde_json::to_vec(&prev_state).unwrap();
      let state = rt.call(&mut prev_state_bytes).await.unwrap();

      let state: Value = serde_json::from_slice(&state).unwrap();
      assert_eq!(state.get("counter").unwrap(), 1);

      assert_eq!(rt.get_cost(), source.0);
    }
  }

  #[test]
  fn test_metering_general() {
    let metering = Metering::new(test_cost_function);
    let module = metering
      .inject(include_bytes!("./testdata/metering/add.wasm"))
      .unwrap();
    // Deterministic codegen.
    assert_eq!(
      &module.finish(),
      include_bytes!("./testdata/metering/add.metering.wasm")
    );
  }

  #[test]
  fn test_metering_nop() {
    let metering = Metering::new(test_cost_function);

    const NOP_WASM: [u8; 8] = [
      0x00, 0x61, 0x73, 0x6D, // Magic
      0x01, 0x00, 0x00, 0x00, // Version
    ];

    let module = metering.inject(&NOP_WASM).unwrap();

    const METERING_NOP_BOILERPLATE: [u8; 35] = [
      0x00, 0x61, 0x73, 0x6D, // Magic
      0x01, 0x00, 0x00, 0x00, // Version
      // ..
      // (module
      // (type $t0 (func (param i32)))
      // (import "3em" "consumeGas" (func $3em.consumeGas (type $t0))))
      // ..
      0x01, 0x05, 0x01, 0x60, 0x01, 0x7F, 0x00, 0x02, 0x12, 0x01, 0x03, 0x33,
      0x65, 0x6D, 0x0A, 0x63, 0x6F, 0x6E, 0x73, 0x75, 0x6D, 0x65, 0x47, 0x61,
      0x73, 0x00, 0x00,
    ];

    assert_eq!(&module.finish(), &METERING_NOP_BOILERPLATE);
  }

  #[test]
  fn test_metering_invalid_module() {
    let metering = Metering::new(test_cost_function);

    let module = metering.inject(&[]);
    assert!(&module.is_err());
  }
}
