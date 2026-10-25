// The runtime for pages compiled by `hypra build --target js`. A compiled page is an ES module whose default
// export is a Page:
//
//     import page from "./product.js";
//     const output = page.render({ product });
//     output.html; output.markdown; output.json; output.webmcp;
//
// Values are shown, escaped and typed exactly as the compiler's static build does, so a page renders the same
// at request time as `hypra build` writes it.

/** Thrown in strict mode when the data doesn't match what a page reads, e.g. a missing field. */
export class DataError extends Error {
  name = "DataError";
}

/** A compiled page. The compiler writes `export default new Page({ ... })`; call `render` with its data. */
export class Page {
  #props;
  #sources;
  #body;
  #meta;

  constructor({ props, sources, body, meta }) {
    this.#props = props;
    this.#sources = sources;
    this.#body = body;
    this.#meta = meta;
  }

  /**
   * Renders the page with its data.
   *
   * - `strict`: throw a DataError for missing props and fields instead of rendering them empty.
   * - `formFields`: a function that returns hidden fields for POST forms to this site, e.g. `{ _csrf: token }`.
   * - `alternateUrl`: where the page's Markdown is served, for `<link rel="alternate">`.
   */
  render(data = {}, { strict = false, formFields = null, alternateUrl = null } = {}) {
    if (strict) {
      for (const [prop, at] of this.#props) {
        if (!has(data, prop)) throw new DataError(`${at}: prop \`${prop}\` is missing from the data`);
      }
    }
    const c = new Context(data, strict, formFields, alternateUrl);
    this.#body(c);
    return Output.assemble(c, this.#meta(c));
  }

  /** The data the page expects, from its `props(...)`. */
  get props() {
    return this.#props.map(([prop]) => prop);
  }

  /** The `.hypra` files the page was compiled from, the page first. */
  get sources() {
    return [...this.#sources];
  }
}

/** Everything one page renders to: HTML for browsers, Markdown for LLMs, JSON for apps, WebMCP tools for agents. */
export class Output {
  constructor(html, markdown, json, webmcp) {
    this.html = html;
    this.markdown = markdown;
    this.json = json;
    this.webmcp = webmcp;
  }

  /**
   * The page as the client asked for it in its Accept header: HTML for browsers, Markdown for `text/markdown`,
   * JSON for `application/json`. Send the body with the content type, and `Vary: Accept`.
   */
  negotiate(accept = "") {
    const format = formatFor(accept);
    if (format === "markdown") return { format, contentType: "text/markdown; charset=utf-8", body: this.markdown };
    if (format === "json") return { format, contentType: "application/json", body: toJson(this.json) };
    return { format, contentType: "text/html; charset=utf-8", body: this.html };
  }

  static assemble(c, meta) {
    let markdown = `${c.md.trimEnd()}\n`;
    if (c.actions.length > 0) {
      markdown += "\n## Actions\n\n";
      for (const a of c.actions) {
        const call = markdownSpan(`${a.name}(${signature(a)})`);
        markdown += `- ${call}: ${markdownText(a.description)} → ${a.method} ${markdownText(a.endpoint)}\n`;
      }
    }
    // a page without a description or canonical URL has none; one from data that's null is empty
    const optional = (value) => (value === undefined ? null : display(value));
    const actions = c.actions.map((a) => ({
      name: a.name,
      description: a.description,
      method: a.method,
      endpoint: a.endpoint,
      params: a.params,
      required: a.required,
    }));
    const tools = c.actions.map((a) => ({
      name: a.name,
      description: a.description,
      inputSchema: { type: "object", properties: a.params, required: a.required },
    }));
    const json = {
      title: display(meta.title),
      description: optional(meta.description),
      lang: display(meta.lang),
      canonical: optional(meta.canonical),
      actions,
    };
    return new Output(c.html, markdown, json, tools);
  }
}

/** The state of one render: output, recorded actions and data access. Compiled pages call these methods. */
class Context {
  html = "";
  md = "";
  actions = [];
  #data;
  #strict;
  #formFields;
  #alternateUrl;

  constructor(data, strict, formFields, alternateUrl) {
    this.#data = data;
    this.#strict = strict;
    this.#formFields = formFields;
    this.#alternateUrl = alternateUrl;
  }

  // the form's URL comes as `action` when it's data; hidden fields like a CSRF token never go to other sites
  formFields(indent, action = null) {
    if (this.#formFields === null || (action !== null && !isSameSite(display(action)))) return;
    for (const [name, value] of Object.entries(this.#formFields())) {
      this.html += `${indent}<input type="hidden" name="${escape(name)}" value="${escape(display(value))}">\n`;
    }
  }

  alternateLink(indent) {
    if (this.#alternateUrl !== null) {
      this.html += `${indent}<link rel="alternate" type="text/markdown" href="${escape(String(this.#alternateUrl))}">\n`;
    }
  }

  htmlText(value) {
    this.html += escape(display(value));
  }

  /** A URL attribute: a URL that would run code is replaced. */
  htmlUrl(value) {
    this.html += escape(safeUrl(display(value)));
  }

  /** Text inside `<script>` or `<style>`, which can't close the element. */
  htmlScript(value) {
    this.html += display(value).replaceAll("</", "<\\/");
  }

  /** Data inside a JSON script: a JSON string that can't close the element. */
  htmlScriptJson(value) {
    this.html += toJson(display(value)).replaceAll("<", "\\u003C").replaceAll(">", "\\u003E").replaceAll("&", "\\u0026");
  }

  /** Unescaped HTML, from `raw`. */
  htmlRaw(value) {
    this.html += display(value);
  }

  htmlJson(value) {
    this.html += escape(toJson(display(value)));
  }

  mdText(value) {
    this.md += markdownText(display(value));
  }

  mdUrl(value) {
    this.md += markdownUrl(display(value));
  }

  /** Text inside a Markdown code span, which stays on one line. */
  mdCode(value) {
    this.md += markdownCode(display(value).replace(/[\r\n]/g, " "));
  }

  mdFence(value) {
    this.md += markdownCode(display(value));
  }

  mdJson(value) {
    this.md += toJson(display(value));
  }

  /** Reads page data, e.g. ["product", "name"] for `product.name`. */
  data(path, at) {
    const [root, ...rest] = path;
    // a missing prop is reported once, as a missing prop
    if (!has(this.#data, root)) return null;
    return this.#walk(root, read(this.#data, root), rest, at);
  }

  /** Reads a field of a loop variable or component parameter. */
  get(value, name, path, at) {
    return this.#walk(name, value, path, at);
  }

  each(list, source, at) {
    if (Array.isArray(list)) return list;
    // a Set or generator is a list too; a string or a Map isn't, as in the static build
    if (isObject(list) && typeof list[Symbol.iterator] === "function" && !(list instanceof Map)) return [...list];
    if (list !== null && list !== undefined) this.#fail(at, `\`${source}\` isn't a list`);
    return [];
  }

  concat(...parts) {
    return parts.map(display).join("");
  }

  equals(value, text) {
    return display(value) === text;
  }

  truthy(value) {
    return truthy(value);
  }

  action(name, description, method, endpoint, params) {
    // fromEntries makes every name an own key, even `__proto__`
    const schemas = Object.fromEntries(params.map((param) => [param.name, schemaOf(param)]));
    const required = params.filter((param) => param.required).map((param) => param.name);
    const existing = this.actions.find((a) => a.name === name);
    if (existing) {
      mergeParams(existing.params, schemas);
      return;
    }
    this.actions.push({ name, description: display(description), method, endpoint: display(endpoint), params: schemas, required });
  }

  #walk(name, current, path, at) {
    let walked = name;
    for (const key of path) {
      if (!has(current, key)) {
        this.#fail(at, `\`${walked}\` has no field \`${key}\``);
        return null;
      }
      current = read(current, key);
      walked += `.${key}`;
    }
    return current;
  }

  #fail(at, message) {
    if (this.#strict) throw new DataError(`${at}: ${message}`);
  }
}

/** A parameter's JSON Schema: a fixed value, or a field's schema with what it's for. */
function schemaOf(param) {
  if ("value" in param) return { type: type(param.value), const: param.value };
  const description = param.description == null ? "" : display(param.description);
  return description ? { ...param.schema, description } : param.schema;
}

/**
 * An action repeated in a loop is one tool; values that differ between the copies become the choices of that
 * parameter, e.g. the ids of the items a remove button is shown for.
 */
function mergeParams(params, more) {
  for (const [key, schema] of Object.entries(more)) {
    const old = params[key];
    if (!isObject(old) || !("const" in schema)) continue;
    const value = schema.const;
    let values;
    if ("const" in old) {
      if (same(old.const, value)) continue;
      values = [old.const];
    } else if (Array.isArray(old.enum)) {
      values = [...old.enum];
    } else {
      continue;
    }
    if (!values.some((v) => same(v, value))) values.push(value);
    params[key] = { type: old.type, enum: values };
  }
}

/** Compact parameter list for the Markdown action index, e.g. `stars: integer 1..5, body?: string ..2000`. */
function signature(action) {
  return Object.entries(action.params)
    .map(([name, schema]) => {
      if ("const" in schema) return `${name}: ${toJson(schema.const)}`;
      let kind = typeof (schema.format ?? schema.type) === "string" ? (schema.format ?? schema.type) : "any";
      if (Array.isArray(schema.enum)) kind = schema.enum.map(display).join("|");
      const low = schema.minimum ?? schema.minLength;
      const high = schema.maximum ?? schema.maxLength;
      if (low !== undefined || high !== undefined) kind += ` ${low === undefined ? "" : display(low)}..${high === undefined ? "" : display(high)}`;
      const optional = action.required.includes(name) ? "" : "?";
      return `${name}${optional}: ${kind}`;
    })
    .join(", ");
}

// --- how values are read, shown and typed: the same as the compiler's static build

const isObject = (value) => value !== null && typeof value === "object";

/** Whether data has a field: an own property, a Map key, or a getter of a class, but never an array index. */
function has(value, key) {
  if (!isObject(value) || Array.isArray(value)) return false;
  if (value instanceof Map) return value.has(key);
  if (Object.hasOwn(value, key)) return true;
  const proto = Object.getPrototypeOf(value);
  return proto !== null && proto !== Object.prototype && key in value && !(key in Object.prototype);
}

function read(value, key) {
  return value instanceof Map ? value.get(key) : value[key];
}

/** The text a value shows as: strings as they are, null as nothing, everything else as JSON. */
export function display(value) {
  if (value === null || value === undefined) return "";
  if (typeof value === "string") return value;
  // a value that turns itself into JSON, like a Date, shows as what it turns into
  if (typeof value.toJSON === "function") return display(value.toJSON());
  return toJson(value);
}

/**
 * JSON written like the compiler writes it: compact, keys in order, and numbers like Rust's: integers as they
 * are, other numbers in scientific notation outside 1e-5 to 1e15, like `1e+20`.
 */
export function toJson(value, inside = new Set()) {
  if (value === null || value === undefined) return "null";
  if (typeof value === "string") return JSON.stringify(value);
  if (typeof value === "number") return number(value);
  if (typeof value === "bigint") return value.toString();
  if (typeof value === "boolean") return value ? "true" : "false";
  if (typeof value.toJSON === "function") return toJson(value.toJSON(), inside);
  // data that contains itself, like a record that links back to its parent, is written once
  if (inside.has(value)) return "null";
  inside.add(value);
  const json =
    Array.isArray(value) || value instanceof Set
      ? `[${[...value].map((v) => toJson(v, inside)).join(",")}]`
      : `{${(value instanceof Map ? [...value] : Object.entries(value)).map(([k, v]) => `${JSON.stringify(String(k))}:${toJson(v, inside)}`).join(",")}}`;
  inside.delete(value);
  return json;
}

function number(n) {
  if (!Number.isFinite(n)) return "null";
  if (Number.isInteger(n) && Math.abs(n) <= Number.MAX_SAFE_INTEGER) return String(n === 0 ? 0 : n);
  const [mantissa, exponent] = n.toExponential().split("e");
  const e = Number(exponent);
  if (e >= 16 || e < -5) return `${mantissa}e${e < 0 ? "-" : "+"}${Math.abs(e)}`;
  return String(n);
}

/** The JSON Schema type of a value. */
function type(value) {
  if (value === null || value === undefined) return "null";
  if (typeof value === "boolean") return "boolean";
  if (typeof value === "bigint") return "integer";
  if (typeof value === "number") return Number.isInteger(value) && Math.abs(value) <= Number.MAX_SAFE_INTEGER ? "integer" : "number";
  if (typeof value === "string") return "string";
  if (Array.isArray(value) || value instanceof Set) return "array";
  return "object";
}

function truthy(value) {
  if (value === null || value === undefined) return false;
  if (typeof value === "boolean") return value;
  if (typeof value === "string") return value !== "";
  if (typeof value === "number") return value !== 0 && !Number.isNaN(value);
  if (typeof value === "bigint") return value !== 0n;
  if (Array.isArray(value)) return value.length > 0;
  if (value instanceof Map || value instanceof Set) return value.size > 0;
  // an empty object is false, as in the static build
  if (Object.getPrototypeOf(value) === Object.prototype || Object.getPrototypeOf(value) === null) return Object.keys(value).length > 0;
  return true;
}

const same = (a, b) => toJson(a) === toJson(b);

/** Escapes text for HTML content and double-quoted attributes. */
export function escape(text) {
  return text.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;").replaceAll('"', "&quot;");
}

/** Browsers ignore whitespace and control characters in a URL scheme, so they're ignored here too. */
function runsCode(url) {
  const scheme = [...url].filter((c) => c.charCodeAt(0) > 0x20 && c !== "\x7f").join("").toLowerCase();
  return ["javascript:", "vbscript:", "data:text/html"].some((bad) => scheme.startsWith(bad));
}

/**
 * URLs that run code when followed are replaced, so data can't turn a link into a script. Characters URLs can't
 * contain (spaces, quotes, braces, …) are percent-encoded, as a browser would when following the link.
 */
export function safeUrl(url) {
  if (runsCode(url)) return "about:blank#blocked";
  let out = "";
  for (const c of url) {
    const code = c.charCodeAt(0);
    out += code <= 0x20 || code === 0x7f || '"<>\\^`{|}'.includes(c) ? `%${code.toString(16).toUpperCase().padStart(2, "0")}` : c;
  }
  return out;
}

/**
 * A URL on this site, the only kind that receives the page's hidden form fields such as a CSRF token. Browsers
 * drop tabs and newlines, skip leading spaces and control characters and read `\` as `/`, so this does too. Then
 * only a URL without a scheme that doesn't start with `//` is on this site.
 */
function isSameSite(url) {
  const cleaned = url.replace(/[\t\n\r]/g, "").replaceAll("\\", "/").replace(/^[\x00-\x20]+/, "");
  if (cleaned.startsWith("//")) return false;
  // a scheme ends with the first `:` that comes before any `/`, `?` or `#`
  const end = cleaned.search(/[:/?#]/);
  return end === -1 || cleaned[end] !== ":";
}

/**
 * Data shown as Markdown text stays text: it can't add a heading such as a fake `## Actions`, a list, a link or
 * HTML. Line breaks become spaces, so it can only start a line where the page starts one.
 */
function markdownText(text) {
  let out = text.replace(/[\r\n]/g, " ").replace(/[\\`[\]<|]/g, "\\$&");
  // what the text starts with could make the line a heading, list item, rule or quote: `# `, `- `, `---`,
  // `1. `, `>`
  const lead = out.length - out.replace(/^[ \t]+/, "").length;
  const at = (i) => out[lead + i] ?? "";
  const endsMarker = (i) => ["", " ", "\t"].includes(at(i));
  const run = (c) => {
    let n = 0;
    while (at(n) === c) n++;
    return n;
  };
  let digits = 0;
  while (/[0-9]/.test(at(digits))) digits++;
  let mark = null;
  if (at(0) === ">") mark = lead;
  else if ("-+*_".includes(at(0)) && at(0) !== "" && (endsMarker(1) || run(at(0)) > 1)) mark = lead;
  else if (at(0) === "#" && endsMarker(run("#"))) mark = lead;
  else if (digits > 0 && ".)".includes(at(digits)) && at(digits) !== "" && endsMarker(digits + 1)) mark = lead + digits;
  if (mark !== null) out = `${out.slice(0, mark)}\\${out.slice(mark)}`;
  return out;
}

/** A Markdown link or image target: a safe URL that can't close the parentheses around it. */
function markdownUrl(url) {
  return safeUrl(url).replaceAll("(", "%28").replaceAll(")", "%29");
}

/** A whole Markdown code span, wider when the text holds a backtick. */
function markdownSpan(text) {
  const code = markdownCode(text.replace(/[\r\n]/g, " "));
  return code.includes("`") ? `\`\` ${code} \`\`` : `\`${code}\``;
}

/**
 * Breaks every run of backticks with a zero-width space, so the text can't close the code span or fence around
 * it. Replacing doesn't overlap, so one pass leaves two backticks of a run of three; the second breaks them.
 */
function markdownCode(text) {
  return text.replaceAll("``", "`​`").replaceAll("``", "`​`");
}

/** Picks the first type in an Accept header, by quality, that a page can be rendered as. */
function formatFor(accept) {
  const types = String(accept)
    .split(",")
    .map((part, index) => {
      const [type, ...params] = part.split(";").map((s) => s.trim().toLowerCase());
      const q = params.find((p) => p.startsWith("q="));
      return { type, q: q === undefined ? 1 : Number(q.slice(2)), index };
    })
    .filter((t) => t.type && t.q > 0)
    .sort((a, b) => b.q - a.q || a.index - b.index);
  for (const { type } of types) {
    if (type === "text/html" || type === "application/xhtml+xml" || type === "*/*") return "html";
    if (type === "text/markdown") return "markdown";
    if (type === "application/json") return "json";
  }
  return "html";
}
