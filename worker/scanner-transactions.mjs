const SCANNER_TRANSACTION_PATTERNS = [
  /^(?:(?:GET|HEAD) )?\/{1,2}[^/?]+\.php(?:[/?]|$)/i,
  /^(?:(?:GET|HEAD) )?\/(?:_ignition|wp-admin|wp-content|wp-includes)(?:[/?]|$)/i,
  /^(?:(?:GET|HEAD) )?\/\*$/,
];

export const isScannerTransaction = (name) =>
  typeof name === "string" && SCANNER_TRANSACTION_PATTERNS.some((pattern) => pattern.test(name));
