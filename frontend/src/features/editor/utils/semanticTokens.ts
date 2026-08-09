import {
  Decoration,
  type DecorationSet,
  EditorView,
  ViewPlugin,
  type ViewUpdate,
} from "@codemirror/view";
import {
  StateEffect,
  StateField,
  type Range,
} from "@codemirror/state";
import {
  LSPPlugin,
  type LSPClientExtension,
} from "@codemirror/lsp-client";

import styles from "./rsgroundTheme.module.sass";

type SemanticTokens = {
  data: number[];
  resultId?: string;
};

type SemanticTokensProvider = {
  legend?: {
    tokenTypes: string[];
    tokenModifiers: string[];
  };
  full?: boolean | { delta?: boolean };
};

const setSemanticTokens = StateEffect.define<DecorationSet>();

const semanticTokenField = StateField.define<DecorationSet>({
  create: () => Decoration.none,

  update(value, transaction) {
    value = value.map(transaction.changes);

    for (const effect of transaction.effects) {
      if (effect.is(setSemanticTokens)) value = effect.value;
    }

    return value;
  },

  provide: (field) => EditorView.decorations.from(field),
});

class SemanticTokenPlugin {
  private timeout: ReturnType<typeof setTimeout> | null = null;
  private requestNumber = 0;
  private initializing: Promise<null> | null = null;

  constructor(private readonly view: EditorView) {
    const plugin = LSPPlugin.get(view);
    if (plugin) this.watchInitialization(plugin.client.initializing);
    this.schedule(250);
  }

  update(update: ViewUpdate) {
    if (update.docChanged) this.schedule(250);
  }

  destroy() {
    if (this.timeout) clearTimeout(this.timeout);
    this.requestNumber++;
  }

  private schedule(delay: number) {
    if (this.timeout) clearTimeout(this.timeout);
    this.timeout = setTimeout(() => {
      this.timeout = null;
      void this.refresh();
    }, delay);
  }

  private async refresh() {
    const plugin = LSPPlugin.get(this.view);
    if (!plugin) return;

    this.watchInitialization(plugin.client.initializing);

    const provider = plugin.client.serverCapabilities
      ?.semanticTokensProvider as SemanticTokensProvider | undefined;
    if (!provider || !provider.legend || provider.full === false) {
      this.schedule(1_000);
      return;
    }

    const requestNumber = ++this.requestNumber;
    const document = this.view.state.doc.toString();

    try {
      plugin.client.sync();
      const response = await plugin.client.request<
        { textDocument: { uri: string } },
        SemanticTokens | null
      >("textDocument/semanticTokens/full", {
        textDocument: { uri: plugin.uri },
      });

      if (
        requestNumber !== this.requestNumber ||
        this.view.state.doc.toString() !== document ||
        !response
      ) {
        return;
      }

      const decorations = decodeSemanticTokens(
        response.data,
        provider.legend.tokenTypes,
        plugin,
        this.view,
      );

      this.view.dispatch({ effects: setSemanticTokens.of(decorations) });
    } catch {
      // Syntax highlighting remains active when the server does not provide
      // semantic tokens or a request is interrupted during reconnect.
    }
  }

  private watchInitialization(initializing: Promise<null>) {
    if (this.initializing === initializing) return;
    this.initializing = initializing;
    void initializing.then(
      () => this.schedule(0),
      () => undefined,
    );
  }
}

const semanticTokenPlugin = ViewPlugin.fromClass(SemanticTokenPlugin);

export function semanticTokensClientExtension(): LSPClientExtension {
  return {
    clientCapabilities: {
      textDocument: {
          semanticTokens: {
          dynamicRegistration: false,
          requests: { range: false, full: { delta: false } },
          tokenTypes: [
            "namespace",
            "type",
            "class",
            "enum",
            "interface",
            "struct",
            "typeParameter",
            "parameter",
            "variable",
            "property",
            "enumMember",
            "event",
            "function",
            "method",
            "macro",
            "label",
            "comment",
            "string",
            "keyword",
            "number",
            "regexp",
            "operator",
            "decorator",
            "lifetime",
            "attribute",
            "builtinType",
            "constParameter",
            "formatSpecifier",
            "punctuation",
          ],
          tokenModifiers: [
            "declaration",
            "definition",
            "readonly",
            "static",
            "deprecated",
            "abstract",
            "async",
            "modification",
            "documentation",
            "defaultLibrary",
          ],
          formats: ["relative"],
          overlappingTokenSupport: false,
          multilineTokenSupport: true,
        },
      },
    },
    editorExtension: [semanticTokenField, semanticTokenPlugin],
  };
}

function decodeSemanticTokens(
  data: number[],
  tokenTypes: string[],
  plugin: LSPPlugin,
  view: EditorView,
) {
  const decorations: Array<Range<Decoration>> = [];
  let line = 0;
  let character = 0;

  for (let index = 0; index + 4 < data.length; index += 5) {
    const deltaLine = data[index];
    const deltaStart = data[index + 1];
    const length = data[index + 2];
    const tokenType = tokenTypes[data[index + 3]];

    line += deltaLine;
    character = deltaLine === 0 ? character + deltaStart : deltaStart;

    if (!tokenType || length <= 0) continue;

    try {
      const from = plugin.fromPosition({ line, character });
      const to = plugin.fromPosition({ line, character: character + length });
      if (from >= to || to > view.state.doc.length) continue;

      const className = semanticClass(tokenType);
      if (className) decorations.push(Decoration.mark({ class: className }).range(from, to));
    } catch {
      // Ignore tokens with positions outside the current document.
    }
  }

  decorations.sort((a, b) => a.from - b.from || a.to - b.to);
  return Decoration.set(decorations, true);
}

function semanticClass(tokenType: string) {
  const normalized = tokenType.replace(/[^a-zA-Z0-9]/g, "");
  const className = styles[`semantic_${normalized}` as keyof typeof styles];
  if (className) return className;

  const aliases: Record<string, keyof typeof styles> = {
    namespace: "semantic_type",
    class: "semantic_type",
    enum: "semantic_type",
    interface: "semantic_type",
    struct: "semantic_type",
    union: "semantic_type",
    trait: "semantic_type",
    module: "semantic_type",
    enumMember: "semantic_constant",
    typeAlias: "semantic_type",
    typeParameter: "semantic_type",
    method: "semantic_function",
    macro: "semantic_function",
    field: "semantic_property",
    property: "semantic_property",
    parameter: "semantic_variable",
    valueParameter: "semantic_variable",
    constParameter: "semantic_variable",
    unresolvedReference: "semantic_variable",
    constant: "semantic_constant",
    const: "semantic_constant",
    static: "semantic_constant",
    builtinType: "semantic_type",
    selfKeyword: "semantic_keyword",
    builtinAttribute: "semantic_attribute",
    decorator: "semantic_attribute",
    attribute: "semantic_attribute",
    lifetime: "semantic_keyword",
    formatSpecifier: "semantic_string",
    punctuation: "semantic_operator",
  };

  const alias = aliases[tokenType];
  return alias ? styles[alias] : null;
}
