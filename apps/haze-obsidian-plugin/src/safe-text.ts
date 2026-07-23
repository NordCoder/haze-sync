const MAX_STATUS_MESSAGE_LENGTH = 220;
const SECRET_PATTERNS: RegExp[] = [
  /\bBearer\s+[A-Za-z0-9._~+/=-]+/giu,
  /\bAuthorization\s*[:=]\s*[^\s,;]+/giu,
  /\btoken\s*[:=]\s*[^\s,;]+/giu,
  /https?:\/\/[^\s/:@]+:[^\s/@]+@/giu,
];

export function sanitizeStatusMessage(message: string, secrets: readonly string[] = []): string {
  let sanitized = message.replace(/[\r\n\t]+/gu, " ").trim();

  for (const pattern of SECRET_PATTERNS) {
    sanitized = sanitized.replace(pattern, "[redacted]");
  }

  for (const secret of secrets) {
    const normalizedSecret = secret.trim();
    if (normalizedSecret.length < 4) {
      continue;
    }

    sanitized = sanitized.replace(new RegExp(escapeRegExp(normalizedSecret), "gu"), "[redacted]");
  }

  if (sanitized.length > MAX_STATUS_MESSAGE_LENGTH) {
    return `${sanitized.slice(0, MAX_STATUS_MESSAGE_LENGTH - 1)}…`;
  }

  return sanitized;
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
}
