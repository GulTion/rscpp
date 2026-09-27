/** @vitest-environment jsdom */
import { describe, it, expect } from "vitest";
import { EditorState } from "@codemirror/state";
import { cpp } from "@codemirror/lang-cpp";
import { forceParsing, syntaxTree } from "@codemirror/language";
import { EditorView } from "@codemirror/view";
import { editorTheme, syntaxColors } from "../src/theme";

describe("syntax highlighting", () => {
  it("parses C++ so highlight styles can apply", () => {
    const host = document.createElement("div");
    document.body.appendChild(host);
    const view = new EditorView({
      parent: host,
      state: EditorState.create({
        doc: "int main() { return 0; }",
        extensions: [cpp(), syntaxColors, editorTheme("light")],
      }),
    });
    forceParsing(view);
    const tree = syntaxTree(view.state);
    expect(tree.length).toBeGreaterThan(0);
    const names: string[] = [];
    tree.iterate({
      enter(n) {
        names.push(n.name);
      },
    });
    expect(names.some((n) => /Type|Keyword|Number|Definition/.test(n))).toBe(
      true,
    );
    view.destroy();
    host.remove();
  });
});
