// Hypra for CodeMirror: the same tokens as the VS Code grammar (editors/vscode/syntaxes/hypra.tmLanguage.json).
import { StreamLanguage } from "@codemirror/language";

const keywords = /^(component|layout|for|in|if|else|not|use)\b/;
const builtins = /^(page|props|head|body|slot|raw|allow|action|field)\b(?=\s*[({"]|\s*$|\s*\/\/)/;
const element = /^[a-z][a-z0-9]*(?:-[a-z0-9]+)*(?=\s*[({"]|\s*$|\s*\/\/)/;
const interpolation = /^\{[a-z_][A-Za-z0-9_]*(?:\.[A-Za-z0-9_]+)*\}/;

export const hypra = StreamLanguage.define({
  name: "hypra",
  startState: () => ({ string: null }),
  token(stream, state) {
    // a """ block ends at a line that starts with """
    if (state.string === '"""') {
      if (stream.sol() && stream.match(/^\s*"""/)) {
        state.string = null;
        return "string";
      }
      if (stream.match(interpolation)) return "variableName.special";
      if (stream.match("{{") || stream.match("}}")) return "escape";
      if (!stream.eatWhile(/[^{}]/)) stream.next();
      return "string";
    }
    if (state.string === '"') {
      if (stream.match(/^\\./)) return "escape";
      if (stream.match(interpolation)) return "variableName.special";
      if (stream.eat('"')) {
        state.string = null;
        return "string";
      }
      if (!stream.eatWhile(/[^"\\{]/)) stream.next();
      if (stream.eol()) state.string = null;
      return "string";
    }
    if (stream.eatSpace()) return null;
    if (stream.match("//")) {
      stream.skipToEnd();
      return "comment";
    }
    if (stream.match('"""')) {
      state.string = '"""';
      stream.skipToEnd();
      return "string";
    }
    if (stream.eat('"')) {
      state.string = '"';
      return "string";
    }
    if (stream.match(keywords)) return "keyword";
    if (stream.match(/^[A-Za-z_][A-Za-z0-9_-]*(?=\s*:(?!:))/)) return "attributeName";
    if (stream.match(builtins)) return "keyword";
    if (stream.match(/^[A-Z][A-Za-z0-9_]*/)) return "typeName";
    if (stream.match(/^(true|false)\b/)) return "bool";
    if (stream.match(element)) return "tagName";
    if (stream.match(/^-?\d+(?:\.\d+)?\b/)) return "number";
    if (stream.match(/^[a-z_][A-Za-z0-9_]*(?:\.[A-Za-z0-9_]+)*/)) return "variableName";
    if (stream.match(/^(==|!=|\.\.|=)/)) return "operator";
    if (stream.match(/^[{}()[\]]/)) return "bracket";
    stream.next();
    return "punctuation";
  },
  languageData: { commentTokens: { line: "//" }, closeBrackets: { brackets: ["(", "[", "{", '"'] } },
});
