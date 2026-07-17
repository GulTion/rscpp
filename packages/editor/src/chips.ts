export function formatChip(value: {
  kind: string;
  value?: unknown;
}): string {
  switch (value.kind) {
    case "Int":
    case "Float":
    case "Bool":
    case "Char":
    case "Str":
      return `⦃${value.value}⦄`;
    case "Object":
      return `⦃#${value.value}⦄`;
    case "Void":
      return "⦃void⦄";
    case "Nullptr":
      return "⦃nullptr⦄";
    default:
      return "⦃…⦄";
  }
}
