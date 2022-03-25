const {
  FunctionDeclaration,
  Source,
  CommonFlags,
  Parser,
  ASTBuilder,
  NodeKind,
} = require("visitor-as/as");

const { utils, TransformVisitor, SimpleParser } = require("visitor-as");

class ThreeEMTransform extends TransformVisitor {
  parser;

  visitFunctionDeclaration(node) {
    if (utils.hasDecorator(node, "contract")) {
      node.name.text = "__inner_impl";
      node.flags = CommonFlags.EXPORT;
    }

    return super.visitFunctionDeclaration(node);
  }

  afterParse(parser) {
    this.parser = parser;
    const p = new Parser(this.parser.diagnostics);

    let sources = this.parser.sources.filter(utils.not(utils.isLibrary));
    let contract = sources.find(
      (source) =>
        source.simplePath !== "index-stub" && utils.isUserEntry(source),
    );

    if (!contract) {
      throw new Error("No contract source found");
    }

    this.parser.sources = this.parser.sources.filter((s) =>
      !utils.isUserEntry(s)
    );
    this.program.sources = this.program.sources.filter((s) =>
      !utils.isUserEntry(s)
    );

    let source = contract.statements.map((node) => {
      if (
        node.kind == NodeKind.FUNCTIONDECLARATION &&
        utils.hasDecorator(node, "contract")
      ) {
        node.name.text = "__inner_impl";
        node.flags = CommonFlags.EXPORT;
      }

      return ASTBuilder.build(node);
    }).join("\n");

    p.parseFile(
      `${source}
export function _alloc(size: usize): usize {
    return heap.alloc(size);
}

let LEN: usize = 0;
export function get_len(): usize {
    return LEN;
}

function read_buf(ptr: usize, size: usize): Uint8Array {
    let buf = new Uint8Array(size);
    for (let i = 0 as usize; i < size; i++) {
    buf[i] += load<u8>(ptr + i);
    }
    return buf;
}

export function handle(
    state_ptr: usize,
    state_size: usize,
    action_ptr: usize,
    action_size: usize,
    contract_info_ptr: usize,
    contract_info_size: usize,
): usize {
    const state = read_buf(state_ptr, state_size);
    const _action = read_buf(action_ptr, action_size);

    let stateObj: JSON.Obj =
    <JSON.Obj> (JSON.parse(String.UTF8.decode(state.buffer)));

    let result = __inner_impl(
        stateObj,
    ).serialize();


    LEN = result.byteLength;

    return result.dataStart;
}
`,
      contract.normalizedPath,
      true,
    );

    let entry = p.sources.pop();
    this.program.sources.push(entry);
    this.parser.sources.push(entry);
    this.visit(sources);
  }
}

module.exports = ThreeEMTransform;
