pub use primitive_types::U256;
pub use primitive_types::H128;
use tiny_keccak::Hasher;
use tiny_keccak::Keccak;

pub mod storage;

pub use storage::Storage;

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

fn to_signed(value: U256) -> U256 {
  match value.bit(255) {
    true => (!value).overflowing_add(U256::one()).0,
    false => value,
  }
}

fn get_window_data(data: &Vec<u8>, window_size: usize, offset: usize) -> Vec<u8> {
  let start_index = offset % data.len();
  let end_index = start_index + window_size;

  // Ensure the end index doesn't go beyond the length of the data
  let end_index = if end_index > data.len() {
      data.len()
  } else {
      end_index
  };

  // Return the data within the specified window
  data[start_index..end_index].to_vec()
}

fn filter_left_zeros(data: Vec<u8>) -> Vec<u8> {
  let mut result = Vec::new();
  let mut found_non_zero = false;

  for &value in &data {
      if value > 0 {
          found_non_zero = true;
      }

      if found_non_zero || value > 0 {
          result.push(value);
      }
  }

  result
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
    Push0 = 0x5f, // Newly Added Opcode
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
    Invalid = 0xfe,
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
    let data = Vec::with_capacity(MAX_STACK_SIZE);

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

  pub fn peek_step(&self, step: usize) -> U256 {
    self.data[self.data.len() - step]
  }

  pub fn swap(&mut self, index: usize) {
    let ptr = self.data.len() - 1;

    // dbg!("Attempting swap ptr = {}, value = {}", ptr, self.data[ptr]);

    if ptr < index {
      return;
    }
    self.data.swap(ptr, ptr - index);

    // dbg!("Swapped {}", self.data[ptr]);
  }

  pub fn dup(&mut self, index: usize) {
    self.push(self.data[self.data.len() - index]);
  }
}

pub struct Machine<'a> {
  pub stack: Stack,
  state: U256,
  memory: Vec<u8>,
  pub result: Vec<u8>,
  // The cost function.
  cost_fn: Box<dyn Fn(&Instruction) -> U256>,
  fetch_contract: Box<dyn Fn(&U256) -> Option<ContractInfo> + 'a>,
  // Total gas used so far.
  // gas_used += cost_fn(instruction)
  gas_used: U256,
  // The input data.
  data: Vec<u8>,
  pub storage: Storage,
  owner: U256,
}

#[derive(PartialEq, Debug)]
pub enum AbortError {
  DivZero,
  InvalidOpcode,
}

#[derive(PartialEq, Debug)]
pub enum ExecutionState {
  Abort(AbortError),
  Revert,
  Ok,
}

#[derive(Default, Clone)]
pub struct BlockInfo {
  pub timestamp: U256,
  pub difficulty: U256,
  pub block_hash: U256,
  pub number: U256,
}

pub struct ContractInfo {
  pub store: Storage,
  pub bytecode: Vec<u8>,
}

impl<'a> Machine<'a> {
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
      fetch_contract: Box::new(|_| None),
      gas_used: U256::zero(),
      data: Vec::new(),
      storage: Storage::new(U256::zero()),
      owner: U256::zero(),
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
      fetch_contract: Box::new(|_| None),
      storage: Storage::new(U256::zero()),
      owner: U256::zero(),
    }
  }

  pub fn set_storage(&mut self, storage: Storage) {
    self.storage = storage;
  }

  pub fn set_fetcher(
    &mut self,
    fetcher: Box<dyn Fn(&U256) -> Option<ContractInfo> + 'a>,
  ) {
    self.fetch_contract = fetcher;
  }

  pub fn execute(
    &mut self,
    bytecode: &[u8],
    block_info: BlockInfo,
  ) -> ExecutionState {
    let mut pc = 0;
    let len = bytecode.len();
    let mut counter = 0;

    while pc < len {
      let opcode = bytecode[pc];
      let inst = match Instruction::try_from(opcode) {
        Ok(inst) => inst,
        Err(_) => {
          return ExecutionState::Abort(AbortError::InvalidOpcode);
        }
      };

      let cost = (self.cost_fn)(&inst);

      pc += 1;
      counter += 1;
      /* 
      if counter == 576 {
         break;
      }
      */
      println!("{:#?}", inst);

      println!("Position: {:#?}", pc);
      println!("Counter: {:#?}", counter);
      println!("===================");
      match inst {
        Instruction::Stop => {}
        Instruction::Add => {
          let lhs = self.stack.pop();
          let rhs = self.stack.pop();
          let lhs16: u16 = lhs.as_u64() as u16;
          let rhs16: u16 = rhs.as_u64() as u16;
          let sum = U256::from(lhs16.overflowing_add(rhs16).0);
          self.stack.push(sum);
        }
        Instruction::Sub => {
          let lhs = self.stack.pop();
          let rhs = self.stack.pop();
          let lhs16: u16 = lhs.as_u64() as u16;
          let rhs16: u16 = rhs.as_u64() as u16;
          let difference = U256::from(lhs16.overflowing_sub(rhs16).0);
          self.stack.push(difference);
        }
        Instruction::Mul => {
          let lhs = self.stack.pop();
          let rhs = self.stack.pop();
          let lhs16: u16 = lhs.as_u64() as u16;
          let rhs16: u16 = rhs.as_u64() as u16;
          let product = U256::from(lhs16.overflowing_mul(rhs16).0);
          self.stack.push(product);
        }
        Instruction::Div => {
          let lhs = self.stack.pop();
          let rhs = self.stack.pop();
          let lhs16: u16 = lhs.as_u64() as u16;
          let rhs16: u16 = rhs.as_u64() as u16;
          let quotient = U256::from(lhs16.overflowing_div(rhs16).0);
          if rhs == U256::zero() {
            return ExecutionState::Abort(AbortError::DivZero);
          }

          self.stack.push(quotient);
        }
        Instruction::SDiv => {
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
          fn to_signed(value: U256) -> U256 {
            match value.bit(255) {
              true => (!value).overflowing_add(U256::one()).0,
              false => value,
            }
          }

          let lhs = self.stack.pop();
          let signed_lhs = to_signed(lhs);
          let sign = lhs.bit(255);

          let rhs = to_signed(self.stack.pop());

          if rhs == U256::zero() {
            self.stack.push(U256::zero());
          } else {
            let value = signed_lhs % rhs;
            self.stack.push(match sign {
              true => (!value).overflowing_add(U256::one()).0,
              false => value,
            });
          }
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
          let pos = self.stack.pop();
          let value = self.stack.pop();

          if pos > U256::from(32) {
            self.stack.push(value);
          } else {
            let bit_pos = (pos.low_u64() * 8 + 7) as usize;
            let bit = value.bit(bit_pos);

            let mask = (U256::one() << bit_pos) - U256::one();
            let result = if bit { value | !mask } else { value & mask };

            self.stack.push(result);
          }
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
          let (lhs, l_sign) = {
            let lhs = self.stack.pop();
            let l_sign = lhs.bit(255);
            (to_signed(lhs), l_sign)
          };

          let (rhs, r_sign) = {
            let rhs = self.stack.pop();
            let r_sign = rhs.bit(255);
            (to_signed(rhs), r_sign)
          };

          let result = match (l_sign, r_sign) {
            (false, false) => lhs < rhs,
            (true, true) => lhs > rhs,
            (true, false) => true,
            (false, true) => false,
          };

          self
            .stack
            .push(if result { U256::one() } else { U256::zero() });
        }
        Instruction::SGt => {
          let (lhs, l_sign) = {
            let lhs = self.stack.pop();
            let l_sign = lhs.bit(255);
            (to_signed(lhs), l_sign)
          };

          let (rhs, r_sign) = {
            let rhs = self.stack.pop();
            let r_sign = rhs.bit(255);
            (to_signed(rhs), r_sign)
          };

          let result = match (l_sign, r_sign) {
            (false, false) => lhs > rhs,
            (true, true) => lhs < rhs,
            (true, false) => false,
            (false, true) => true,
          };

          self
            .stack
            .push(if result { U256::one() } else { U256::zero() });
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

        Instruction::Shl => {
          let rhs = self.stack.pop();
          let lhs = self.stack.pop();

          if rhs < U256::from(256) {
            self.stack.push(lhs << rhs);
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
          let val16: u16 = val.as_u64() as u16;
          let not_val16 = U256::from(!val16);
          self.stack.push(not_val16);
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
          self.stack.push(self.owner);
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
          let mem_offset = self.stack.pop();

          let offset = self.stack.pop();
          let size = self.stack.pop();

          if offset > U256::from(self.data.len())
            || offset.overflowing_add(size).1
          {
            return ExecutionState::Ok;
          }

          let offset = offset.low_u64() as usize;
          let size = size.low_u64() as usize;
          let end = std::cmp::min(offset + size, self.data.len());
          let mut data = self.data[offset..end].to_vec();
          data.resize(32, 0u8);

          let mem_offset = mem_offset.low_u64() as usize;
          self.memory[mem_offset..mem_offset + 32]
            .copy_from_slice(data.as_slice());
        }
        Instruction::CodeSize => {
          self.stack.push(U256::from(len));
        }
        Instruction::CodeCopy => {
          let mem_offset = self.stack.peek_step(1).low_u64() as usize;
          let code_offset = self.stack.peek_step(2);
          let len = self.stack.peek_step(3).low_u64() as usize;
          if code_offset > usize::max_value().into() {
            dbg!("CODECOPY: offset too large");
          }
          let code_offset = code_offset.low_u64() as usize;
          
          let code: &[u8];
          // Needed for testing parts of bytecode to avoid out of bound errors in &bytecode[code_offset..code_offset + len]
          let mut temp_vec: Vec<u8> = vec![]; 
          
          if code_offset + len <= bytecode.len() {
            code = &bytecode[code_offset..code_offset + len];
          } else {
            // Used for byte by byte testing
            temp_vec.extend(std::iter::repeat(0).take(len - code_offset));
            code = &temp_vec;
          }
          println!("length of new code {:#?}", &code.len());
          //ATTENTION: Later investigate if code is being injected into memory correctly
          // I dont think the below code is correct.
          if self.memory.len() < mem_offset + 32 {
            self.memory.resize(mem_offset + 32, 0);
          }

          // Calculate padding for resizing
          let current_size = code.len() + mem_offset;
          let remainder = current_size % 32;
          let padding = if remainder == 0 { 0 } else { 32 - remainder };
          
          // Resize
          self.memory.resize(current_size + padding + 32, 0);
          
          //Calculate new space of zeroes
          for i in 0..=code.len() - 1 {
            if i > code.len() {
              self.memory[mem_offset + i] = 0;
            } else {
              self.memory[mem_offset + i] = code[i];
            }
          }
        }
        Instruction::ExtCodeSize => {
          // Fetch the `Contract-Src` from Arweave for the contract.
        }
        Instruction::ExtCodeCopy => {
          // Fetch the `Contract-Src` from Arweave for the contract.
        }
        Instruction::ReturnDataSize => {
          self.stack.push(U256::from(self.result.len()));
        }
        Instruction::ReturnDataCopy => {
          let mem_offset = self.stack.pop().low_u64() as usize;
          let data_offset = self.stack.pop().low_u64() as usize;
          let length = self.stack.pop().low_u64() as usize;

          if self.result.len() < data_offset + length {
            panic!("Return data out of bounds");
          }

          let data = &self.result[data_offset..data_offset + length];

          if self.memory.len() < mem_offset + 32 {
            self.memory.resize(mem_offset + 32, 0);
          }

          for i in 0..32 {
            if i > data.len() {
              self.memory[mem_offset + i] = 0;
            } else {
              self.memory[mem_offset + i] = data[i];
            }
          }
        }
        Instruction::BlockHash => {
          self.stack.push(block_info.block_hash);
        }
        Instruction::Timestamp => {
          self.stack.push(block_info.timestamp);
        }
        Instruction::Number => {
          self.stack.push(block_info.number);
        }
        Instruction::Difficulty => {
          self.stack.push(block_info.difficulty);
        }
        Instruction::GasLimit => {
          self.stack.push(U256::MAX);
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
          println!("offset: {:#?}", len);

          // Calcuate bytes to add to memory based on offset
          let num_memory_rows = self.memory.len() / 32;
          let offset_needed_rows = ((len + 32) as f64 / 32.0).ceil() as usize;
          let rows_to_add = offset_needed_rows - num_memory_rows;
  
          if rows_to_add > 0 {
            for _ in 0..=rows_to_add - 1 {
              self.memory.extend(std::iter::repeat(0).take(32));
            }
          }

          let word = get_window_data(&self.memory, 32, len);
          let filtered_word = filter_left_zeros(word);
          println!("filtered word: {:#?}", filtered_word);
          
          /* 
          for (idx, mem_ptr) in (0..len).zip(len..len + 32) {
            data[idx] = self.memory[mem_ptr];
          }
          println!("altered data: {:#?}", data.as_slice());
          self.stack.push(U256::from(data.as_slice()));
          */
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
            self.memory.resize(mem_ptr + 32, 0);
          }

          self.memory[mem_ptr] = val.byte(0);
        }
        Instruction::SLoad => {
          let offset = self.stack.pop();
          let data = self.storage.get(&self.owner, &offset);
          self.stack.push(data);
        }
        Instruction::SStore => {
          let offset = self.stack.pop();
          let val = self.stack.pop();
          self.storage.insert(&self.owner, offset, val);
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
        Instruction::MSize
        | Instruction::Gas
        | Instruction::GasPrice
        | Instruction::Coinbase => {
          self.stack.push(U256::zero());
        }
        Instruction::JumpDest => {}
        Instruction::Push0 => {
          self.stack.push(U256::from(0x00));
        }
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
        | Instruction::Log4 => {}
        Instruction::Create => {
          // TODO
        }
        Instruction::Call => {
          // Call parameters
          let _gas = self.stack.pop();
          let addr = self.stack.pop();
          let _value = self.stack.pop();
          let in_offset = self.stack.pop().low_u64() as usize;
          let in_size = self.stack.pop().low_u64() as usize;
          let out_offset = self.stack.pop().low_u64() as usize;
          let out_size = self.stack.pop().low_u64() as usize;

          let input = &bytecode[in_offset..in_offset + in_size];

          let mut evm = Self::new_with_data(|_| U256::zero(), input.to_vec());
          let contract = (self.fetch_contract)(&addr)
            .expect("No fetch contract handler provided.");
          evm.set_storage(contract.store);

          evm.execute(&contract.bytecode, block_info.clone());

          self.memory[out_offset..out_offset + out_size]
            .copy_from_slice(&evm.result);
        }
        Instruction::CallCode | Instruction::DelegateCall => {
          // Call parameters
          let _gas = self.stack.pop();
          let addr = self.stack.pop();
          let in_offset = self.stack.pop().low_u64() as usize;
          let in_size = self.stack.pop().low_u64() as usize;
          let out_offset = self.stack.pop().low_u64() as usize;
          let out_size = self.stack.pop().low_u64() as usize;

          let input = &bytecode[in_offset..in_offset + in_size];

          let mut evm = Self::new_with_data(|_| U256::zero(), input.to_vec());
          let contract = (self.fetch_contract)(&addr)
            .expect("No fetch contract handler provided.");
          evm.set_storage(contract.store);

          evm.execute(&contract.bytecode, block_info.clone());

          self.memory[out_offset..out_offset + out_size]
            .copy_from_slice(&evm.result);
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
        Instruction::Invalid => {
          // revisit this logic. Similar to Revert but must consume all gas
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
  use crate::storage::Storage;
  use crate::ExecutionState;
  use crate::Instruction;
  use crate::Machine;
  use crate::Stack;

  use hex_literal::hex;
  use primitive_types::U256;

  fn test_cost_fn(_: &Instruction) -> U256 {
    U256::zero()
  }
  /*
  #[allow(dead_code)]
  fn print_vm_memory(vm: &Machine) {
    let mem = &vm.memory;
    //println!("{:?}", mem);
    for (i, cell) in mem.iter().enumerate() {
      if i % 16 == 0 {
        print!("\n{:x}: ", i);
      }

      print!("{:#04x} ", cell);
    }
  }
  */
  /*
   #[test]
   fn test_basic() {
     let mut machine = Machine::new(test_cost_fn);

     let status = machine.execute(
       &[
         Instruction::Push1 as u8,
         0x01,
         Instruction::Push1 as u8,
         0x02,
         Instruction::Add as u8,
       ],
       Default::default(),
     );

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

     let status = machine.execute(
       &[
         Instruction::Push1 as u8,
         0x00,
         Instruction::Push1 as u8,
         0x03,
         Instruction::Swap1 as u8,
         Instruction::Pop as u8,
         Instruction::Swap1 as u8,
       ],
       Default::default(),
     );

     assert_eq!(status, ExecutionState::Ok);
     assert_eq!(machine.stack.pop(), U256::from(0x03));
   }

   #[test]
   fn test_sdiv() {
     let mut machine = Machine::new(test_cost_fn);

     let status = machine.execute(
       &[
         Instruction::Push1 as u8,
         0x02,
         Instruction::Push1 as u8,
         0x04,
         Instruction::SDiv as u8,
       ],
       Default::default(),
     );

     assert_eq!(status, ExecutionState::Ok);
     assert_eq!(machine.stack.pop(), U256::from(0x02));
   }

   #[test]
   fn test_add_solidity() {
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

     let status = machine.execute(&hex_code, Default::default());
     assert_eq!(status, ExecutionState::Ok);

     assert_eq!(machine.result.len(), 32);
     assert_eq!(machine.result.pop(), Some(0x03));
   }

   #[test]
   fn test_erc_twenty() {
     let hex_code = hex!("608060405234801561001057600080fd5b50600436106100365760003560e01c80632e64cec11461003b5780636057361d14610059575b600080fd5b610043610075565b60405161005091906100a1565b60405180910390f35b610073600480360381019061006e91906100ed565b61007e565b005b60008054905090565b8060008190555050565b6000819050919050565b61009b81610088565b82525050565b60006020820190506100b66000830184610092565b92915050565b600080fd5b6100ca81610088565b81146100d557600080fd5b50565b6000813590506100e7816100c1565b92915050565b600060208284031215610103576101026100bc565b5b6000610111848285016100d8565b9150509291505056fea2646970667358221220322c78243e61b783558509c9cc22cb8493dde6925aa5e89a08cdf6e22f279ef164736f6c63430008120033");
     let mut machine =
       Machine::new_with_data(test_cost_fn, hex!("6057361d0000000000000000000000000000000000000000000000000000000000000002").to_vec());

     let status = machine.execute(&hex_code, Default::default());

     //println!("{:#?}", machine.storage);
     //assert_eq!(status, ExecutionState::Ok);

     //assert_eq!(machine.result.len(), 32);
     //assert_eq!(machine.result.pop(), Some(0x03));
   }

   #[test]
   fn test_mstore() {
     //
     let mut machine = Machine::new(test_cost_fn);

     // memory[0x40:0x60] = 0x80
     let status = machine.execute(
       &[
         Instruction::Push1 as u8,
         0x80,
         Instruction::Push1 as u8,
         0x40,
         Instruction::MStore as u8,
       ],
       Default::default(),
     );
     assert_eq!(status, ExecutionState::Ok);
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

     let status = machine.execute(&bytes, Default::default());
     assert_eq!(status, ExecutionState::Ok);
   }

   #[test]
   fn test_storage_constructor() {
     let bytes = hex!("608060405234801561001057600080fd5b506040518060400160405280600a81526020017f6c6974746c6564697679000000000000000000000000000000000000000000008152506000908051906020019061005c929190610062565b50610166565b82805461006e90610134565b90600052602060002090601f01602090048101928261009057600085556100d7565b82601f106100a957805160ff19168380011785556100d7565b828001600101855582156100d7579182015b828111156100d65782518255916020019190600101906100bb565b5b5090506100e491906100e8565b5090565b5b808211156101015760008160009055506001016100e9565b5090565b7f4e487b7100000000000000000000000000000000000000000000000000000000600052602260045260246000fd5b6000600282049050600182168061014c57607f821691505b602082108114156101605761015f610105565b5b50919050565b6104a8806101756000396000f3fe608060405234801561001057600080fd5b50600436106100365760003560e01c80633a525c291461003b5780636b701e0814610059575b600080fd5b610043610075565b604051610050919061025d565b60405180910390f35b610073600480360381019061006e91906103c8565b610107565b005b60606000805461008490610440565b80601f01602080910402602001604051908101604052809291908181526020018280546100b090610440565b80156100fd5780601f106100d2576101008083540402835291602001916100fd565b820191906000526020600020905b8154815290600101906020018083116100e057829003601f168201915b5050505050905090565b806000908051906020019061011d929190610121565b5050565b82805461012d90610440565b90600052602060002090601f01602090048101928261014f5760008555610196565b82601f1061016857805160ff1916838001178555610196565b82800160010185558215610196579182015b8281111561019557825182559160200191906001019061017a565b5b5090506101a391906101a7565b5090565b5b808211156101c05760008160009055506001016101a8565b5090565b600081519050919050565b600082825260208201905092915050565b60005b838110156101fe5780820151818401526020810190506101e3565b8381111561020d576000848401525b50505050565b6000601f19601f8301169050919050565b600061022f826101c4565b61023981856101cf565b93506102498185602086016101e0565b61025281610213565b840191505092915050565b600060208201905081810360008301526102778184610224565b905092915050565b6000604051905090565b600080fd5b600080fd5b600080fd5b600080fd5b7f4e487b7100000000000000000000000000000000000000000000000000000000600052604160045260246000fd5b6102d582610213565b810181811067ffffffffffffffff821117156102f4576102f361029d565b5b80604052505050565b600061030761027f565b905061031382826102cc565b919050565b600067ffffffffffffffff8211156103335761033261029d565b5b61033c82610213565b9050602081019050919050565b82818337600083830152505050565b600061036b61036684610318565b6102fd565b90508281526020810184848401111561038757610386610298565b5b610392848285610349565b509392505050565b600082601f8301126103af576103ae610293565b5b81356103bf848260208601610358565b91505092915050565b6000602082840312156103de576103dd610289565b5b600082013567ffffffffffffffff8111156103fc576103fb61028e565b5b6104088482850161039a565b91505092915050565b7f4e487b7100000000000000000000000000000000000000000000000000000000600052602260045260246000fd5b6000600282049050600182168061045857607f821691505b6020821081141561046c5761046b610411565b5b5091905056fea264697066735822122007a3fec27bf391246bb4a62e66c81e304129cd8c6427df54eb8e9cebec9c658f64736f6c634300080a0033");
     let mut machine = Machine::new(test_cost_fn);
     let status = machine.execute(&bytes, Default::default());
     assert_eq!(status, ExecutionState::Ok);
   }
  */
  #[test]
  fn test_erc_constructor() {
    let bytes = hex!("7f00000000000000000000000000000000000000000000000000000000000000ff600052600351");
    let mut machine = Machine::new(test_cost_fn);
    let status = machine.execute(&bytes, Default::default());
    //assert_eq!(status, ExecutionState::Ok);

    println!("LOOK DOWN");
    println!("Result: {:#?}", machine.result);
    println!("Storage: {:#?}", machine.storage);
    println!("Memory: {:#?}", machine.memory);
    println!("Stack: {:#?}", machine.stack);
  }
  /*
  #[test]
  fn test_erc_twenty() {
    let hex_code = hex!("6080604052348015600e575f80fd5b50600436106026575f3560e01c8063eea32eb214602a575b5f80fd5b60306044565b604051603b91906062565b60405180910390f35b5f8054905090565b5f819050919050565b605c81604c565b82525050565b5f60208201905060735f8301846055565b9291505056fea2646970667358221220c90818a724b5acfd11bea9df587e8ad68deeee49ede9e80140caace0f5608ee464736f6c63430008140033");
    let mut machine =
      Machine::new_with_data(test_cost_fn, hex!("eea32eb2").to_vec());

    let status = machine.execute(&hex_code, Default::default());

    println!("STOR: {:#?}", machine.storage);
    println!("RES: {:#?}", machine.result);
    //assert_eq!(status, ExecutionState::Ok);

    //assert_eq!(machine.result.len(), 32);
    //assert_eq!(machine.result.pop(), Some(0x03));
  }
  */
  /*
  #[test]
  fn test_storage_retrieve() {
    let bytes = hex!("608060405234801561001057600080fd5b50600436106100365760003560e01c80633a525c291461003b5780636b701e0814610059575b600080fd5b610043610075565b6040516100509190610259565b60405180910390f35b610073600480360381019061006e91906102ea565b610107565b005b60606000805461008490610366565b80601f01602080910402602001604051908101604052809291908181526020018280546100b090610366565b80156100fd5780601f106100d2576101008083540402835291602001916100fd565b820191906000526020600020905b8154815290600101906020018083116100e057829003601f168201915b5050505050905090565b81816000919061011892919061011d565b505050565b82805461012990610366565b90600052602060002090601f01602090048101928261014b5760008555610192565b82601f1061016457803560ff1916838001178555610192565b82800160010185558215610192579182015b82811115610191578235825591602001919060010190610176565b5b50905061019f91906101a3565b5090565b5b808211156101bc5760008160009055506001016101a4565b5090565b600081519050919050565b600082825260208201905092915050565b60005b838110156101fa5780820151818401526020810190506101df565b83811115610209576000848401525b50505050565b6000601f19601f8301169050919050565b600061022b826101c0565b61023581856101cb565b93506102458185602086016101dc565b61024e8161020f565b840191505092915050565b600060208201905081810360008301526102738184610220565b905092915050565b600080fd5b600080fd5b600080fd5b600080fd5b600080fd5b60008083601f8401126102aa576102a9610285565b5b8235905067ffffffffffffffff8111156102c7576102c661028a565b5b6020830191508360018202830111156102e3576102e261028f565b5b9250929050565b600080602083850312156103015761030061027b565b5b600083013567ffffffffffffffff81111561031f5761031e610280565b5b61032b85828601610294565b92509250509250929050565b7f4e487b7100000000000000000000000000000000000000000000000000000000600052602260045260246000fd5b6000600282049050600182168061037e57607f821691505b6020821081141561039257610391610337565b5b5091905056fea26469706673582212207d39b40255f686f82c889c56bfa8000e6be27070f005d24ec016e786e6ce64fc64736f6c634300080a0033");
    let mut machine =
      Machine::new_with_data(test_cost_fn, hex!("3a525c29").to_vec());

    let account = U256::zero();
    let mut storage = Storage::new(account);
    storage.insert(
      &account,
      U256::zero(),
      U256::from(
        "0x6c6974746c656469767900000000000000000000000000000000000000000014",
      ),
    );
    machine.set_storage(storage);

    let status = machine.execute(&bytes, Default::default());
    assert_eq!(status, ExecutionState::Ok);

    assert_eq!(machine.result.len(), 96);

    let len = U256::from(&machine.result[32..64]).low_u64() as usize;

    let result_string = &machine.result[64..64 + len];
    assert_eq!(std::str::from_utf8(result_string).unwrap(), "littledivy");
  }
  */
}
/*
Yield integer overflow error to u64
60017fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff01
*/