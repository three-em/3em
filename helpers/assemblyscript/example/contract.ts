import { JSON, JSONEncoder } from "assemblyscript-json";

@contract
export function handle(state: JSON.Obj): JSONEncoder {
  const counter: JSON.Integer | null = state.getInteger("counter");

  let encoder = new JSONEncoder();
  encoder.pushObject(null);
  if (counter === null) {
    encoder.setInteger("counter", 0);
  } else {
    encoder.setInteger("counter", (counter.valueOf() + 1) as i64);
  }
  
  encoder.popObject();
  return encoder;
}
