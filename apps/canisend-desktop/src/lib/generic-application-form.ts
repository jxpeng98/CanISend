export function exactUtf8Span(
  source: string,
  statement: string,
): [number, number] | null {
  const start = source.indexOf(statement);
  if (start < 0 || !statement) return null;
  const encoder = new TextEncoder();
  const startByte = encoder.encode(source.slice(0, start)).length;
  return [startByte, startByte + encoder.encode(statement).length];
}
