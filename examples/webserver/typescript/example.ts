import { RawClient } from './generated/client'
import { WebsocketClient } from 'jsonrpc'

run().catch(console.error)

async function run() {
  const transport = new WebsocketClient("ws://localhost:20808/ws");
  const client = new RawClient(transport)
  let res: unknown;
  res = await client.manyArgs(3, ["foo", "bar"]);
  console.log("manyArgs", res);
  res = await client.yell("hello");
  console.log("yell", res);
  res = await client.sum({ a: 3, b: 7 });
  console.log("sum", res);
}
