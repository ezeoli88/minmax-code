export type ToolResultMeta =
  | { type: "edit_file"; path: string; oldStr: string; newStr: string }
  | { type: "write_file"; path: string; content: string; isNew: boolean };
