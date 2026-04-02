import { finalizeMessage } from "./message.js";

const hostMessage = await Deno.core.ops.op_runtime_message("HELLO");
globalThis.RUNTIME_SMOKE_RESULT = await finalizeMessage(hostMessage);
