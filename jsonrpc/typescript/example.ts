import { WebsocketClient } from "./client.js";
run();
async function run(): Promise<void> {
  const client = new WebsocketClient("ws://localhost:20808/ws");
  let res: unknown;
  res = await client.manyArgs(3, ["foo", "bar"]);
  console.log("manyArgs", res);
  res = await client.yell("hello");
  console.log("yell", res);
  res = await client.sum({ a: 3, b: 7 });
  console.log("sum", res);
}
