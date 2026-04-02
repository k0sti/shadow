export async function finalizeMessage(hostMessage) {
  const suffix = await Promise.resolve("AND FILE MODULE");
  return `${hostMessage} ${suffix}`;
}
