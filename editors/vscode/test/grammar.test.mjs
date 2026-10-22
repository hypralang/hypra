// Tokenizes Hypra with the grammar, the way VS Code does, and checks what each piece is highlighted as.
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import assert from "node:assert/strict";
import oniguruma from "vscode-oniguruma";
import textmate from "vscode-textmate";

const require = createRequire(import.meta.url);
const wasm = readFileSync(require.resolve("vscode-oniguruma/release/onig.wasm")).buffer;
await oniguruma.loadWASM(wasm);

const registry = new textmate.Registry({
  onigLib: Promise.resolve({
    createOnigScanner: (patterns) => new oniguruma.OnigScanner(patterns),
    createOnigString: (s) => new oniguruma.OnigString(s),
  }),
  // other languages, like the JavaScript inside a script, aren't part of this test
  loadGrammar: async (scope) =>
    scope === "source.hypra"
      ? textmate.parseRawGrammar(readFileSync(new URL("../syntaxes/hypra.tmLanguage.json", import.meta.url), "utf8"), "hypra.json")
      : null,
});
const grammar = await registry.loadGrammar("source.hypra");

/** Each piece of the source with the most specific scope it gets. */
function tokens(source) {
  const out = [];
  let state = textmate.INITIAL;
  for (const line of source.split("\n")) {
    const result = grammar.tokenizeLine(line, state);
    for (const t of result.tokens) {
      const text = line.slice(t.startIndex, t.endIndex);
      if (text.trim()) out.push([text, t.scopes.at(-1)]);
    }
    state = result.ruleStack;
  }
  return out;
}

function expect(source, expected) {
  const actual = tokens(source);
  for (const [text, scope] of expected) {
    const found = actual.find(([t]) => t === text);
    assert.ok(found, `"${text}" isn't a token of: ${source}\n${JSON.stringify(actual)}`);
    assert.equal(found[1], scope, `"${text}" in: ${source}`);
  }
}

expect(`// a comment`, [["// a comment", "comment.line.double-slash.hypra"]]);
expect(`use "layouts/shop.hypra"`, [["use", "keyword.control.import.hypra"]]);
expect(`component Card(title: Text, featured: Bool = false) {`, [
  ["component", "storage.type.hypra"],
  ["Card", "entity.name.type.hypra"],
  ["title", "entity.other.attribute-name.hypra"],
  ["Text", "entity.name.type.hypra"],
  ["=", "keyword.operator.hypra"],
  ["false", "constant.language.boolean.hypra"],
]);
expect(`  a("/shoes", class: "link") "Shoes {count}"`, [
  ["a", "entity.name.tag.hypra"],
  ["class", "entity.other.attribute-name.hypra"],
  ["count", "variable.other.hypra"],
]);
expect(`  button("@click": "open = true", hx-post: "/like") "Like"`, [
  ["button", "entity.name.tag.hypra"],
  ['"@click"', "entity.other.attribute-name.hypra"],
  ["hx-post", "entity.other.attribute-name.hypra"],
]);
expect(`  for item in cart.items { li "{item.name}" }`, [
  ["for", "keyword.control.loop.hypra"],
  ["item", "variable.other.hypra"],
  ["cart.items", "variable.other.hypra"],
  ["li", "entity.name.tag.hypra"],
]);
expect(`  if featured { p "x" } else if user.role == "admin" { p "y" }`, [
  ["if", "keyword.control.conditional.hypra"],
  ["featured", "variable.other.hypra"],
  ["==", "keyword.operator.hypra"],
  ["p", "entity.name.tag.hypra"],
]);
expect(`  field("age", type: Int(1..5), label: "Age")`, [
  ["field", "keyword.other.element.hypra"],
  ["Int", "entity.name.type.hypra"],
  ["1", "constant.numeric.hypra"],
  ["..", "keyword.operator.hypra"],
]);
expect(`  p "say \\"hi\\" and {{not data}}"`, [
  ['\\"', "constant.character.escape.hypra"],
  ["{{", "constant.character.escape.hypra"],
]);
expect(`  pre """\n    {post.title} stays data\n    """\n  p "after"`, [
  ["post.title", "variable.other.hypra"],
  ["after", "string.quoted.double.hypra"],
]);
expect(`  script(type: "module") """\n    const x = {a: 1};\n    """\n  h1 "after"`, [
  ["script", "entity.name.tag.hypra"],
  ["h1", "entity.name.tag.hypra"],
]);
expect(`  script(type: "application/ld+json") """\n    {"offer": {"price": {product.price}}}\n    """\n  article(class: "x")`, [
  ["product.price", "variable.other.hypra"],
  ["article", "entity.name.tag.hypra"],
]);

console.log("grammar ok");
