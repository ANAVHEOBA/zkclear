import nacl from "tweetnacl";

export interface PlainIntentInput {
  side: "buy" | "sell";
  asset_pair: string;
  amount: string;
  limit_price: string;
  settlement_currency: string;
  counterparty_id: string;
}

export interface BuiltIntent {
  encryptedPayload: string;
  signature: string;
  signerPublicKey: string;
  nonce: string;
  timestamp: number;
}

const DEFAULT_DEMO_KEY_HEX =
  "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";

export async function buildIntentPayload(input: PlainIntentInput): Promise<BuiltIntent> {
  const payload = JSON.stringify({
    side: input.side,
    asset_pair: input.asset_pair,
    amount: input.amount,
    limit_price: input.limit_price,
    settlement_currency: input.settlement_currency,
    counterparty_id: input.counterparty_id,
  });

  const timestamp = Math.floor(Date.now() / 1000);
  const nonce = crypto.randomUUID();
  const encryptedPayload = await encryptPayload(payload);

  const signer = nacl.sign.keyPair();
  const message = `${encryptedPayload}:${nonce}:${timestamp}`;
  const signatureBytes = nacl.sign.detached(utf8(message), signer.secretKey);

  return {
    encryptedPayload,
    signature: toHex(signatureBytes),
    signerPublicKey: toHex(signer.publicKey),
    nonce,
    timestamp,
  };
}

async function encryptPayload(plain: string): Promise<string> {
  const keyHex =
    process.env.NEXT_PUBLIC_INTENT_ENCRYPTION_KEY_HEX?.trim() || DEFAULT_DEMO_KEY_HEX;
  const key = hexToBytes(keyHex);
  if (key.length !== 32) {
    throw new Error("NEXT_PUBLIC_INTENT_ENCRYPTION_KEY_HEX must be 32-byte hex");
  }

  const cryptoKey = await crypto.subtle.importKey(
    "raw",
    asArrayBuffer(key),
    { name: "AES-GCM" },
    false,
    ["encrypt"],
  );
  const nonce = crypto.getRandomValues(new Uint8Array(12));
  const cipher = await crypto.subtle.encrypt(
    { name: "AES-GCM", iv: nonce },
    cryptoKey,
    asArrayBuffer(utf8(plain)),
  );

  const out = new Uint8Array(nonce.length + cipher.byteLength);
  out.set(nonce, 0);
  out.set(new Uint8Array(cipher), nonce.length);
  return toBase64(out);
}

function utf8(value: string): Uint8Array {
  return new TextEncoder().encode(value);
}

function hexToBytes(hex: string): Uint8Array {
  const clean = hex.startsWith("0x") ? hex.slice(2) : hex;
  if (clean.length % 2 !== 0) {
    throw new Error("invalid hex length");
  }
  const out = new Uint8Array(clean.length / 2);
  for (let i = 0; i < clean.length; i += 2) {
    out[i / 2] = parseInt(clean.slice(i, i + 2), 16);
  }
  return out;
}

function toHex(data: Uint8Array): string {
  return Array.from(data)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

function toBase64(data: Uint8Array): string {
  let bin = "";
  for (let i = 0; i < data.length; i++) {
    bin += String.fromCharCode(data[i]);
  }
  return btoa(bin);
}

function asArrayBuffer(data: Uint8Array): ArrayBuffer {
  return data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength) as ArrayBuffer;
}
