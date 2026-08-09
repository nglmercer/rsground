import { describe, expect, it } from "vitest";

import { FileNodeKind, FileExplorerNode } from "../types";
import { mergeFiles } from "./mergeFiles";

function file(fullPath: string): FileExplorerNode {
  return {
    kind: FileNodeKind.File as const,
    data: {
      fullPath,
      filename: fullPath.split("/").at(-1)!,
    },
  };
}

function folder(
  name: string,
  children: FileExplorerNode[] = [],
): FileExplorerNode {
  return {
    kind: FileNodeKind.Folder as const,
    data: {
      name,
      fullPath: name,
      children,
    },
  };
}

describe("mergeFiles", () => {
  it("builds a sorted tree with folders before files", () => {
    const result = mergeFiles(
      ["z.rs", "src/main.rs", "src/lib.rs", "docs/readme.md"],
      [],
    );

    expect(result.map((node) => node.data.fullPath)).toEqual([
      "docs",
      "src",
      "z.rs",
    ]);

    const docs = result[0];
    const src = result[1];
    expect(docs.kind).toBe(FileNodeKind.Folder);
    expect(src.kind).toBe(FileNodeKind.Folder);
    if (docs.kind === FileNodeKind.Folder && src.kind === FileNodeKind.Folder) {
      expect(docs.data.children.map((node) => node.data.fullPath)).toEqual([
        "docs/readme.md",
      ]);
      expect(src.data.children.map((node) => node.data.fullPath)).toEqual([
        "src/lib.rs",
        "src/main.rs",
      ]);
    }
  });

  it("preserves empty folders from the previous tree and applies prefixes", () => {
    const empty = folder("empty");
    const oldNonEmpty = folder("removed", [file("removed/old.rs")]);

    const result = mergeFiles(["main.rs"], [empty, oldNonEmpty], "project/");

    expect(result).toEqual([
      empty,
      {
        kind: FileNodeKind.File,
        data: { fullPath: "project/main.rs", filename: "main.rs" },
      },
    ]);
  });

  it("reuses old children while merging a folder", () => {
    const oldFolder = folder("src", [file("src/old.rs")]);
    const result = mergeFiles(["src/new.rs"], [oldFolder]);

    expect(result).toHaveLength(1);
    expect(result[0].kind).toBe(FileNodeKind.Folder);
    if (result[0].kind === FileNodeKind.Folder) {
      expect(result[0].data.children).toEqual([
        {
          kind: FileNodeKind.File,
          data: { fullPath: "src/new.rs", filename: "new.rs" },
        },
      ]);
    }
  });
});
