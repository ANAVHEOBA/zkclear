export function getBackendBaseUrl(): string {
  const base = process.env.NEXT_PUBLIC_BACKEND_URL?.trim();
  if (base) {
    return base.replace(/\/+$/, "");
  }
  return "http://127.0.0.1:8080";
}
