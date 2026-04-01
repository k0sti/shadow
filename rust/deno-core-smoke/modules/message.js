export async function buildMessage() {
  const prefix = await Promise.resolve("HELLO");
  return `${prefix} FROM FILE MODULE`;
}
