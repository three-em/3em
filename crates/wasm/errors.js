globalThis.error = function(ptr, len) {
  // `exports` is defined at runtime creation.
  const mem = exports.memory.buffer;
  const buf = new Uint8Array(mem);
  const str = String.fromCharCode.apply(null, buf.subarray(ptr, ptr + len));
  throw new Error(str);
};

