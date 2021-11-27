use primitive_types::U256;
use tiny_keccak::Hasher;
use tiny_keccak::Keccak;

macro_rules! repr_u8 {
  ($(#[$meta:meta])* $vis:vis enum $name:ident {
      $($(#[$vmeta:meta])* $vname:ident $(= $val:expr)?,)*
  }) => {
      $(#[$meta])*
      $vis enum $name {
          $($(#[$vmeta])* $vname $(= $val)?,)*
      }

      impl std::convert::TryFrom<u8> for $name {
          type Error = ();

          fn try_from(v: u8) -> Result<Self, Self::Error> {
              match v {
                  $(x if x == $name::$vname as u8 => Ok($name::$vname),)*
                  _ => Err(()),
              }
          }
      }
  }
}

repr_u8! {
  // EVM instructions
  #[repr(u8)]
  #[derive(Debug, Eq, PartialEq)]
  pub enum Instruction {
    Stop = 0x00,
    Add = 0x01,
    Mul = 0x02,
    Sub = 0x03,
    Div = 0x04,
    SDiv = 0x05,
    Mod = 0x06,
    SMod = 0x07,
    AddMod = 0x08,
    MulMod = 0x09,
    Exp = 0x0a,
    SignExtend = 0x0b,
    // 0x0c - 0x0f reserved
    Lt = 0x10,
    Gt = 0x11,
    SLt = 0x12,
    SGt = 0x13,
    Eq = 0x14,
    IsZero = 0x15,
    And = 0x16,
    Or = 0x17,
    Xor = 0x18,
    Not = 0x19,
    Byte = 0x1a,
    // EIP145
    // https://github.com/ethereum/EIPs/blob/master/EIPS/eip-145.md
    Shl = 0x1b,
    Shr = 0x1c,
    Sar = 0x1d,
    Keccak256 = 0x20,
    // 0x21 - 0x2f reserved
    Address = 0x30,
    Balance = 0x31,
    Origin = 0x32,
    Caller = 0x33,
    CallValue = 0x34,
    CallDataLoad = 0x35,
    CallDataSize = 0x36,
    CallDataCopy = 0x37,
    CodeSize = 0x38,
    CodeCopy = 0x39,
    GasPrice = 0x3a,
    ExtCodeSize = 0x3b,
    ExtCodeCopy = 0x3c,
    ReturnDataSize = 0x3d,
    ReturnDataCopy = 0x3e,
    BlockHash = 0x40,
    Coinbase = 0x41,
    Timestamp = 0x42,
    Number = 0x43,
    Difficulty = 0x44,
    GasLimit = 0x45,
    // EIP 1344
    // https://github.com/ethereum/EIPs/blob/master/EIPS/eip-1344.md
    ChainId = 0x46,
    // 0x47 - 0x4f reserved
    // EIP-3198
    BaseFee = 0x48,
    Pop = 0x50,
    MLoad = 0x51,
    MStore = 0x52,
    MStore8 = 0x53,
    SLoad = 0x54,
    SStore = 0x55,
    Jump = 0x56,
    JumpI = 0x57,
    GetPc = 0x58,
    MSize = 0x59,
    Gas = 0x5a,
    JumpDest = 0x5b,
    // 0x5c - 0x5f reserved
    Push1 = 0x60,
    Push2 = 0x61,
    Push3 = 0x62,
    Push4 = 0x63,
    Push5 = 0x64,
    Push6 = 0x65,
    Push7 = 0x66,
    Push8 = 0x67,
    Push9 = 0x68,
    Push10 = 0x69,
    Push11 = 0x6a,
    Push12 = 0x6b,
    Push13 = 0x6c,
    Push14 = 0x6d,
    Push15 = 0x6e,
    Push16 = 0x6f,
    Push17 = 0x70,
    Push18 = 0x71,
    Push19 = 0x72,
    Push20 = 0x73,
    Push21 = 0x74,
    Push22 = 0x75,
    Push23 = 0x76,
    Push24 = 0x77,
    Push25 = 0x78,
    Push26 = 0x79,
    Push27 = 0x7a,
    Push28 = 0x7b,
    Push29 = 0x7c,
    Push30 = 0x7d,
    Push31 = 0x7e,
    Push32 = 0x7f,
    Dup1 = 0x80,
    Dup2 = 0x81,
    Dup3 = 0x82,
    Dup4 = 0x83,
    Dup5 = 0x84,
    Dup6 = 0x85,
    Dup7 = 0x86,
    Dup8 = 0x87,
    Dup9 = 0x88,
    Dup10 = 0x89,
    Dup11 = 0x8a,
    Dup12 = 0x8b,
    Dup13 = 0x8c,
    Dup14 = 0x8d,
    Dup15 = 0x8e,
    Dup16 = 0x8f,
    Swap1 = 0x90,
    Swap2 = 0x91,
    Swap3 = 0x92,
    Swap4 = 0x93,
    Swap5 = 0x94,
    Swap6 = 0x95,
    Swap7 = 0x96,
    Swap8 = 0x97,
    Swap9 = 0x98,
    Swap10 = 0x99,
    Swap11 = 0x9a,
    Swap12 = 0x9b,
    Swap13 = 0x9c,
    Swap14 = 0x9d,
    Swap15 = 0x9e,
    Swap16 = 0x9f,
    Log0 = 0xa0,
    Log1 = 0xa1,
    Log2 = 0xa2,
    Log3 = 0xa3,
    Log4 = 0xa4,
    // 0xa5 - 0xaf reserved
    Create = 0xf0,
    Call = 0xf1,
    CallCode = 0xf2,
    Return = 0xf3,
    DelegateCall = 0xf4,
    Create2 = 0xfb,
    Revert = 0xfd,
    StaticCall = 0xfa,
    SelfDestruct = 0xff,
  }
}

pub const MAX_STACK_SIZE: usize = 1024;

#[derive(Debug)]
pub struct Stack {
  pub data: Vec<U256>,
}

impl Default for Stack {
  fn default() -> Self {
    let mut data = Vec::with_capacity(MAX_STACK_SIZE);

    Stack { data }
  }
}

impl Stack {
  pub fn push(&mut self, value: U256) {
    self.data.push(value);
  }

  pub fn pop(&mut self) -> U256 {
    self.data.pop().unwrap()
  }

  pub fn peek(&self) -> U256 {
    self.data[self.data.len() - 1]
  }

  pub fn swap(&mut self, index: usize) {
    let ptr = self.data.len() - 1;

    dbg!("Attempting swap ptr = {}, value = {}", ptr, self.data[ptr]);

    if ptr < index {
      return;
    }
    self.data.swap(ptr, ptr - index);

    dbg!("Swapped {}", self.data[ptr]);
  }

  pub fn dup(&mut self, index: usize) {
    self.push(self.data[self.data.len() - index]);
  }
}

pub struct Machine {
  pub stack: Stack,
  state: U256,
  memory: Vec<u8>,
  result: Vec<u8>,
  // The cost function.
  cost_fn: Box<dyn Fn(&Instruction) -> U256>,
  // Total gas used so far.
  // gas_used += cost_fn(instruction)
  gas_used: U256,
  // The input data.
  // <- 4 bytes -> | < 32 bytes -> | ... |
  data: Vec<u8>,
}

#[derive(PartialEq, Debug)]
pub enum AbortError {
  DivZero,
  InvalidOpcode,
}

#[derive(PartialEq, Debug)]
pub enum ExecutionState {
  Abort(AbortError),
  Return(U256),
  Revert,
  Ok,
}

impl Machine {
  pub fn new<T>(cost_fn: T) -> Self
  where
    T: Fn(&Instruction) -> U256 + 'static,
  {
    Machine {
      stack: Stack::default(),
      state: U256::zero(),
      memory: Vec::new(),
      result: Vec::new(),
      cost_fn: Box::new(cost_fn),
      gas_used: U256::zero(),
      data: Vec::new(),
    }
  }

  pub fn new_with_data<T>(cost_fn: T, data: Vec<u8>) -> Self
  where
    T: Fn(&Instruction) -> U256 + 'static,
  {
    Machine {
      stack: Stack::default(),
      state: U256::zero(),
      memory: Vec::new(),
      result: Vec::new(),
      cost_fn: Box::new(cost_fn),
      gas_used: U256::zero(),
      data,
    }
  }

  pub fn execute(&mut self, bytecode: &[u8]) -> ExecutionState {
    let mut pc = 0;
    let len = bytecode.len();
    while pc < len {
      let opcode = bytecode[pc];
      let inst = match Instruction::try_from(opcode) {
        Ok(inst) => inst,
        // For ASSERT (0xfe) and friends.
        Err(_) => {
          return ExecutionState::Abort(AbortError::InvalidOpcode);
        }
      };

      let cost = (self.cost_fn)(&inst);

      pc += 1;

      match inst {
        Instruction::Stop => {}
        Instruction::Add => {
          let lhs = self.stack.pop();
          let rhs = self.stack.pop();
          self.stack.push(lhs + rhs);
        }
        Instruction::Sub => {
          let lhs = self.stack.pop();
          let rhs = self.stack.pop();
          self.stack.push(lhs - rhs);
        }
        Instruction::Mul => {
          let lhs = self.stack.pop();
          let rhs = self.stack.pop();
          self.stack.push(lhs * rhs);
        }
        Instruction::Div => {
          let lhs = self.stack.pop();
          let rhs = self.stack.pop();

          if rhs == U256::zero() {
            return ExecutionState::Abort(AbortError::DivZero);
          }

          self.stack.push(lhs / rhs);
        }
        Instruction::SDiv => {
          fn to_signed(value: U256) -> U256 {
            match value.bit(255) {
              true => (!value).overflowing_add(U256::one()).0,
              false => value,
            }
          }

          let dividend = to_signed(self.stack.pop());
          let divisor = to_signed(self.stack.pop());
          const U256_ZERO: U256 = U256::zero();

          let quotient = if divisor == U256_ZERO {
            U256_ZERO
          } else {
            let min = (U256::one() << 255) - U256::one();
            if dividend == min && divisor == !U256::one() {
              min
            } else {
              let sign = dividend.bit(255) ^ divisor.bit(255);
              match sign {
                true => !(dividend / divisor),
                false => dividend / divisor,
              }
            }
          };

          self.stack.push(quotient);
        }
        Instruction::Mod => {
          let lhs = self.stack.pop();
          let rhs = self.stack.pop();

          let res = match lhs.checked_rem(rhs) {
            Some(res) => res,
            None => U256::zero(),
          };

          self.stack.push(res);
        }
        Instruction::SMod => {
          // TODO
        }
        Instruction::AddMod => {
          let a = self.stack.pop();
          let b = self.stack.pop();
          let c = self.stack.pop();

          let res = match a.checked_add(b) {
            Some(res) => res % c,
            None => U256::zero(),
          };

          self.stack.push(res);
        }
        Instruction::MulMod => {
          let a = self.stack.pop();
          let b = self.stack.pop();
          let c = self.stack.pop();

          let res = match a.checked_mul(b) {
            Some(res) => res % c,
            None => U256::zero(),
          };

          self.stack.push(res);
        }
        Instruction::Exp => {
          let lhs = self.stack.pop();
          let rhs = self.stack.pop();

          self.stack.push(lhs.overflowing_pow(rhs).0)
        }
        Instruction::SignExtend => {
          // TODO
        }
        Instruction::Lt => {
          let lhs = self.stack.pop();
          let rhs = self.stack.pop();

          self
            .stack
            .push(if lhs < rhs { U256::one() } else { U256::zero() });
        }
        Instruction::Gt => {
          let lhs = self.stack.pop();
          let rhs = self.stack.pop();

          self
            .stack
            .push(if lhs > rhs { U256::one() } else { U256::zero() });
        }
        Instruction::SLt => {
          // TODO
        }
        Instruction::SGt => {
          // TODO
        }
        Instruction::Shr => {
          let rhs = self.stack.pop();
          let lhs = self.stack.pop();

          if rhs < U256::from(256) {
            self.stack.push(lhs >> rhs);
          } else {
            self.stack.push(U256::zero());
          }
        }
        Instruction::Eq => {
          let lhs = self.stack.pop();
          let rhs = self.stack.pop();

          self.stack.push(if lhs == rhs {
            U256::one()
          } else {
            U256::zero()
          });
        }
        Instruction::IsZero => {
          let val = self.stack.pop();

          self.stack.push(if val == U256::zero() {
            U256::one()
          } else {
            U256::zero()
          });
        }
        Instruction::And => {
          let lhs = self.stack.pop();
          let rhs = self.stack.pop();

          self.stack.push(lhs & rhs);
        }
        Instruction::Or => {
          let lhs = self.stack.pop();
          let rhs = self.stack.pop();

          self.stack.push(lhs | rhs);
        }
        Instruction::Xor => {
          let lhs = self.stack.pop();
          let rhs = self.stack.pop();

          self.stack.push(lhs ^ rhs);
        }
        Instruction::Not => {
          let val = self.stack.pop();

          self.stack.push(!val);
        }
        Instruction::Byte => {
          let rhs = self.stack.pop();
          let lhs = self.stack.pop();

          match rhs > U256::from(32) {
            true => {
              self.stack.push(U256::zero());
            }
            false => {
              let byte = lhs.byte(rhs.as_u64() as usize);
              self.stack.push(U256::from(byte));
            }
          }
        }
        Instruction::Keccak256 => {
          let offset = self.stack.pop().low_u64() as usize;
          let size = self.stack.pop().low_u64() as usize;

          let data = &self.memory[offset..offset + size];
          let mut result = [0u8; 32];
          let mut keccak = Keccak::v256();

          keccak.update(data);
          keccak.finalize(&mut result);

          self.stack.push(U256::from(result));
        }
        Instruction::Address => {
          // TODO: address
          self.stack.push(U256::zero());
        }
        Instruction::Balance => {
          let _addr = self.stack.pop();
          // TODO: balance
          self.stack.push(U256::zero());
        }
        Instruction::Origin => {
          // TODO: origin
          self.stack.push(U256::zero());
        }
        Instruction::Caller => {
          // TODO: caller
          self.stack.push(U256::zero());
        }
        Instruction::CallValue => {
          self.stack.push(self.state);
        }
        Instruction::CallDataLoad => {
          let offset = self.stack.pop();

          let offset = match offset > usize::max_value().into() {
            true => self.data.len(),
            false => offset.low_u64() as usize,
          };

          let end = std::cmp::min(offset + 32, self.data.len());
          let mut data = self.data[offset..end].to_vec();
          data.resize(32, 0u8);
          self.stack.push(U256::from(data.as_slice()));
        }
        Instruction::CallDataSize => {
          self.stack.push(U256::from(self.data.len()));
        }
        Instruction::CallDataCopy => {
          // TODO
        }
        Instruction::CodeSize => {
          self.stack.push(U256::from(len));
        }
        Instruction::CodeCopy => {
          let mem_offset = self.stack.pop().low_u64() as usize;
          let code_offset = self.stack.pop();
          let len = self.stack.pop().low_u64() as usize;

          if code_offset > usize::max_value().into() {
            dbg!("CODECOPY: offset too large");
          }

          let code_offset = code_offset.low_u64() as usize;
          if code_offset < self.data.len() {
            let code = &bytecode[code_offset..code_offset + len];

            if self.memory.len() < mem_offset + 32 {
              self.memory.resize(mem_offset + 32, 0);
            }

            for i in 0..32 {
              if i > code.len() {
                self.memory[mem_offset + i] = 0;
              } else {
                self.memory[mem_offset + i] = code[i];
              }
            }
          }
        }
        Instruction::GasPrice => {
          // TODO: Gas
          self.stack.push(U256::zero());
        }
        Instruction::ExtCodeSize => {
          // TODO
        }
        Instruction::ExtCodeCopy => {
          // TODO
        }
        Instruction::ReturnDataSize => {
          // TODO
        }
        Instruction::ReturnDataCopy => {
          // TODO
        }
        Instruction::BlockHash => {
          // TODO
        }
        Instruction::Coinbase => {
          // TODO
        }
        Instruction::Timestamp => {
          // TODO
        }
        Instruction::Number => {
          // TODO
        }
        Instruction::Difficulty => {
          // TODO
        }
        Instruction::GasLimit => {
          // TODO
        }
        Instruction::Pop => {
          self.stack.pop();
        }
        Instruction::MLoad => {
          let offset = self.stack.pop();
          if offset > usize::max_value().into() {
            dbg!("MLOAD: offset too large");
          }
          let len = offset.low_u64() as usize;
          let mut data = vec![0u8; 32];

          for (idx, mem_ptr) in (0..len).zip(len..len + 32) {
            data[idx] = self.memory[mem_ptr];
          }

          self.stack.push(U256::from(data.as_slice()));
        }
        Instruction::MStore => {
          let offset = self.stack.pop();
          let val = self.stack.pop();
          if offset > usize::max_value().into() {
            dbg!("MStore: offset too large");
          }
          let offset = offset.low_u64() as usize;
          if self.memory.len() <= offset + 32 {
            self.memory.resize(offset + 32, 0);
          }

          for i in 0..32 {
            let mem_ptr = offset + i;

            // Big endian byte
            let index = 4 * 8 - 1 - i;
            self.memory[mem_ptr] = val.byte(index);
          }
        }
        Instruction::MStore8 => {
          let offset = self.stack.pop();
          let val = self.stack.pop();
          if offset > usize::max_value().into() {
            dbg!("MStore8: offset too large");
          }
          let mem_ptr = offset.low_u64() as usize;
          if mem_ptr >= self.memory.len() {
            self.memory.resize(mem_ptr + 1, 0);
          }

          self.memory[mem_ptr] = val.byte(0);
        }
        Instruction::SLoad => {
          // TODO
        }
        Instruction::SStore => {
          // TODO
        }
        Instruction::Jump => {
          let offset = self.stack.pop();
          pc = offset.low_u64() as usize;
        }
        Instruction::JumpI => {
          let offset = self.stack.pop();
          let condition = self.stack.pop();
          if condition != U256::zero() {
            pc = offset.low_u64() as usize;
          }
        }
        Instruction::GetPc => {
          self.stack.push(U256::from(pc));
        }
        Instruction::MSize => {
          // TODO
        }
        Instruction::Gas => {
          // TODO: remaining gas
          self.stack.push(U256::zero());
        }
        Instruction::JumpDest => {}
        Instruction::Push1
        | Instruction::Push2
        | Instruction::Push3
        | Instruction::Push4
        | Instruction::Push5
        | Instruction::Push6
        | Instruction::Push7
        | Instruction::Push8
        | Instruction::Push9
        | Instruction::Push10
        | Instruction::Push11
        | Instruction::Push12
        | Instruction::Push13
        | Instruction::Push14
        | Instruction::Push15
        | Instruction::Push16
        | Instruction::Push17
        | Instruction::Push18
        | Instruction::Push19
        | Instruction::Push20
        | Instruction::Push21
        | Instruction::Push22
        | Instruction::Push23
        | Instruction::Push24
        | Instruction::Push25
        | Instruction::Push26
        | Instruction::Push27
        | Instruction::Push28
        | Instruction::Push29
        | Instruction::Push30
        | Instruction::Push31
        | Instruction::Push32 => {
          let value_size = (opcode - 0x60 + 1) as usize;

          let value = &bytecode[pc..pc + value_size];
          pc += value_size;

          self.stack.push(U256::from(value));
        }
        Instruction::Dup1
        | Instruction::Dup2
        | Instruction::Dup3
        | Instruction::Dup4
        | Instruction::Dup5
        | Instruction::Dup6
        | Instruction::Dup7
        | Instruction::Dup8
        | Instruction::Dup9
        | Instruction::Dup10
        | Instruction::Dup11
        | Instruction::Dup12
        | Instruction::Dup13
        | Instruction::Dup14
        | Instruction::Dup15
        | Instruction::Dup16 => {
          let size = (opcode - 0x80 + 1) as usize;

          self.stack.dup(size);
        }
        Instruction::Swap1
        | Instruction::Swap2
        | Instruction::Swap3
        | Instruction::Swap4
        | Instruction::Swap5
        | Instruction::Swap6
        | Instruction::Swap7
        | Instruction::Swap8
        | Instruction::Swap9
        | Instruction::Swap10
        | Instruction::Swap11
        | Instruction::Swap12
        | Instruction::Swap13
        | Instruction::Swap14
        | Instruction::Swap15
        | Instruction::Swap16 => {
          let size = (opcode - 0x90 + 1) as usize;

          self.stack.swap(size);
        }
        Instruction::Log0
        | Instruction::Log1
        | Instruction::Log2
        | Instruction::Log3
        | Instruction::Log4 => {
          // TODO
        }
        Instruction::Create => {
          // TODO
        }
        Instruction::Call
        | Instruction::CallCode
        | Instruction::DelegateCall => {
          // TODO
        }
        Instruction::Return => {
          let offset = self.stack.pop();

          if offset > usize::max_value().into() {
            dbg!("Return: offset too large");
          }
          let offset = offset.low_u64() as usize;
          let size = self.stack.pop().low_u64() as usize;

          let mut data = vec![];
          for idx in offset..offset + size {
            if idx >= self.memory.len() {
              data.push(0);
            } else {
              data.push(self.memory[idx]);
            }
          }

          self.result = data;
          break;
        }
        Instruction::Revert => {
          return ExecutionState::Revert;
        }
        _ => unimplemented!(),
      }

      self.gas_used += cost;
    }

    ExecutionState::Ok
  }
}

#[cfg(test)]
mod tests {
  use crate::runtime::evm::ExecutionState;
  use crate::runtime::evm::Instruction;
  use crate::runtime::evm::Machine;
  use crate::runtime::evm::Stack;

  use hex_literal::hex;
  use primitive_types::U256;

  fn test_cost_fn(_: &Instruction) -> U256 {
    U256::zero()
  }

  fn print_vm_memory(vm: &Machine) {
    let mem = &vm.memory;
    println!("{:?}", mem);
    for i in 0..mem.len() {
      if i % 16 == 0 {
        print!("\n{:x}: ", i);
      }

      print!("{:#04x} ", mem[i]);
    }
  }

  #[test]
  fn test_basic() {
    let mut machine = Machine::new(test_cost_fn);

    let status = machine.execute(&[
      Instruction::Push1 as u8,
      0x01,
      Instruction::Push1 as u8,
      0x02,
      Instruction::Add as u8,
    ]);

    assert_eq!(status, ExecutionState::Ok);
    assert_eq!(machine.stack.pop(), U256::from(0x03));
  }

  #[test]
  fn test_stack_swap() {
    let mut stack = Stack::default();

    stack.push(U256::from(0x01));
    stack.push(U256::from(0x02));
    stack.swap(1);

    stack.pop();
    stack.swap(1);

    assert_eq!(stack.pop(), U256::from(0x02));
  }

  #[test]
  fn test_swap_jump() {
    let mut machine = Machine::new(test_cost_fn);

    let status = machine.execute(&[
      Instruction::Push1 as u8,
      0x00,
      Instruction::Push1 as u8,
      0x03,
      Instruction::Swap1 as u8,
      Instruction::Pop as u8,
      Instruction::Swap1 as u8,
    ]);

    assert_eq!(status, ExecutionState::Ok);
    assert_eq!(machine.stack.pop(), U256::from(0x03));
  }

  #[test]
  fn test_sdiv() {
    let mut machine = Machine::new(test_cost_fn);

    let status = machine.execute(&[
      Instruction::Push1 as u8,
      0x02,
      Instruction::Push1 as u8,
      0x04,
      Instruction::SDiv as u8,
    ]);

    assert_eq!(status, ExecutionState::Ok);
    assert_eq!(machine.stack.pop(), U256::from(0x02));
  }

  #[test]
  fn test_add_solidity() {
    let mut machine = Machine::new(test_cost_fn);
    // label_0000:
    // 	// Inputs[1] { @0005  msg.value }
    // 	0000    60  PUSH1 0x80
    // 	0002    60  PUSH1 0x40
    // 	0004    52  MSTORE
    // 	0005    34  CALLVALUE
    // 	0006    80  DUP1
    // 	0007    15  ISZERO
    // 	0008    60  PUSH1 0x0f
    // 	000A    57  *JUMPI
    // 	// Stack delta = +1
    // 	// Outputs[2]
    // 	// {
    // 	//     @0004  memory[0x40:0x60] = 0x80
    // 	//     @0005  stack[0] = msg.value
    // 	// }
    // 	// Block ends with conditional jump to 0x000f, if !msg.value

    // label_000B:
    // 	// Incoming jump from 0x000A, if not !msg.value
    // 	// Inputs[1] { @000E  memory[0x00:0x00] }
    // 	000B    60  PUSH1 0x00
    // 	000D    80  DUP1
    // 	000E    FD  *REVERT
    // 	// Stack delta = +0
    // 	// Outputs[1] { @000E  revert(memory[0x00:0x00]); }
    // 	// Block terminates

    // label_000F:
    // 	// Incoming jump from 0x000A, if !msg.value
    // 	// Inputs[1] { @001B  memory[0x00:0x77] }
    // 	000F    5B  JUMPDEST
    // 	0010    50  POP
    // 	0011    60  PUSH1 0x77
    // 	0013    80  DUP1
    // 	0014    60  PUSH1 0x1d
    // 	0016    60  PUSH1 0x00
    // 	0018    39  CODECOPY
    // 	0019    60  PUSH1 0x00
    // 	001B    F3  *RETURN
    // 	// Stack delta = -1
    // 	// Outputs[2]
    // 	// {
    // 	//     @0018  memory[0x00:0x77] = code[0x1d:0x94]
    // 	//     @001B  return memory[0x00:0x77];
    // 	// }
    // 	// Block terminates

    // 	001C    FE    *ASSERT
    // 	001D    60    PUSH1 0x80
    // 	001F    60    PUSH1 0x40
    // 	0021    52    MSTORE
    // 	0022    34    CALLVALUE
    // 	0023    80    DUP1
    // 	0024    15    ISZERO
    // 	0025    60    PUSH1 0x0f
    // 	0027    57    *JUMPI
    // 	0028    60    PUSH1 0x00
    // 	002A    80    DUP1
    // 	002B    FD    *REVERT
    // 	002C    5B    JUMPDEST
    // 	002D    50    POP
    // 	002E    60    PUSH1 0x04
    // 	0030    36    CALLDATASIZE
    // 	0031    10    LT
    // 	0032    60    PUSH1 0x28
    // 	0034    57    *JUMPI
    // 	0035    60    PUSH1 0x00
    // 	0037    35    CALLDATALOAD
    // 	0038    60    PUSH1 0xe0
    // 	003A    1C    SHR
    // 	003B    80    DUP1
    // 	003C    63    PUSH4 0x4f2be91f
    // 	0041    14    EQ
    // 	0042    60    PUSH1 0x2d
    // 	0044    57    *JUMPI
    // 	0045    5B    JUMPDEST
    // 	0046    60    PUSH1 0x00
    // 	0048    80    DUP1
    // 	0049    FD    *REVERT
    // 	004A    5B    JUMPDEST
    // 	004B    60    PUSH1 0x03
    // 	004D    60    PUSH1 0x40
    // 	004F    51    MLOAD
    // 	0050    90    SWAP1
    // 	0051    81    DUP2
    // 	0052    52    MSTORE
    // 	0053    60    PUSH1 0x20
    // 	0055    01    ADD
    // 	0056    60    PUSH1 0x40
    // 	0058    51    MLOAD
    // 	0059    80    DUP1
    // 	005A    91    SWAP2
    // 	005B    03    SUB
    // 	005C    90    SWAP1
    // 	005D    F3    *RETURN
    // 	005E    FE    *ASSERT
    // 	005F    A2    LOG2
    // 	0060    64    PUSH5 0x6970667358
    // 	0066    22    22
    // 	0067    12    SLT
    // 	0068    20    SHA3
    // 	0069    FD    *REVERT
    // 	006A    A5    A5
    // 	006B    D9    D9
    // 	006C    D2    D2
    // 	006D    16    AND
    // 	006E    9A    SWAP11
    // 	006F    B4    B4
    // 	0070    47    SELFBALANCE
    // 	0071    B0    PUSH
    // 	0072    A2    LOG2
    // 	0073    35    CALLDATALOAD
    // 	0074    7F    PUSH32 0xe5d46648452a2ffa2d4046a757ef013fbfe7a7d764736f6c634300080a0033
    let hex_code = hex!("6080604052348015600f57600080fd5b506004361060285760003560e01c80634f2be91f14602d575b600080fd5b60336047565b604051603e91906067565b60405180910390f35b60006003905090565b6000819050919050565b6061816050565b82525050565b6000602082019050607a6000830184605a565b9291505056fea26469706673582212200047574855cc88b41f29d7879f8126fe8da6f03c5f30c66c8e1290510af5253964736f6c634300080a0033");
    let mut machine =
      Machine::new_with_data(test_cost_fn, hex!("4f2be91f").to_vec());

    let status = machine.execute(&hex_code);
    assert_eq!(status, ExecutionState::Ok);

    assert_eq!(machine.result.len(), 32);
    assert_eq!(machine.result.pop(), Some(0x03));
  }

  #[test]
  fn test_mstore() {
    //
    let mut machine = Machine::new(test_cost_fn);

    // memory[0x40:0x60] = 0x80
    let status = machine.execute(&[
      Instruction::Push1 as u8,
      0x80,
      Instruction::Push1 as u8,
      0x40,
      Instruction::MStore as u8,
    ]);
  }

  #[test]
  fn test_keccak256() {
    // object "object" {
    //   code {
    //       mstore(0, 0x10)
    //       pop(keccak256(0, 0x20))
    //   }
    // }
    let bytes = hex!("6010600052602060002050");
    let mut machine = Machine::new(test_cost_fn);

    let status = machine.execute(&bytes);
    assert_eq!(status, ExecutionState::Ok);
  }
}
