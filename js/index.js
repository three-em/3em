if (typeof process == "object") {
  throw new Error("Use `@three-em/node` instead.")
}

export * from "./executor.js";