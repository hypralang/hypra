// Types for the runtime of pages compiled by `hypra build --target js`.

/** Thrown in strict mode when the data doesn't match what a page reads, e.g. a missing field. */
export class DataError extends Error {}

export interface RenderOptions {
  /** Throw a DataError for missing props and fields instead of rendering them empty. */
  strict?: boolean;
  /** Hidden fields for POST forms to this site, e.g. `() => ({ _csrf: token })`. */
  formFields?: (() => Record<string, unknown>) | null;
  /** Where the page's Markdown is served, for `<link rel="alternate" type="text/markdown">`. */
  alternateUrl?: string | null;
}

/** A compiled page: the default export of a module written by `hypra build --target js`. */
export class Page {
  /** Renders the page with its data: an object or a Map with the page's props. */
  render(data?: object, options?: RenderOptions): Output;
  /** The data the page expects, from its `props(...)`. */
  readonly props: string[];
  /** The `.hypra` files the page was compiled from, the page first. */
  readonly sources: string[];
}

/** A JSON Schema, as Hypra writes it for an action's parameters. */
export type Schema = Record<string, unknown>;

export interface Action {
  name: string;
  description: string;
  method: string;
  endpoint: string;
  params: Record<string, Schema>;
  required: string[];
}

export interface Tool {
  name: string;
  description: string;
  inputSchema: { type: "object"; properties: Record<string, Schema>; required: string[] };
}

export interface Negotiated {
  format: "html" | "markdown" | "json";
  contentType: string;
  body: string;
}

/** Everything one page renders to. */
export class Output {
  /** The page, for browsers. */
  readonly html: string;
  /** The page as Markdown, for LLMs, with a list of its actions. */
  readonly markdown: string;
  /** The page's metadata and actions, for apps. */
  readonly json: {
    title: string;
    description: string | null;
    lang: string;
    canonical: string | null;
    actions: Action[];
  };
  /** WebMCP tool definitions, for AI agents. */
  readonly webmcp: Tool[];
  /** The page as an Accept header asks for it: HTML, Markdown for `text/markdown`, JSON for `application/json`. */
  negotiate(accept?: string | null): Negotiated;
}

/** The text a value shows as in a page: strings as they are, null as nothing, everything else as JSON. */
export function display(value: unknown): string;
/** JSON written the way the compiler writes it. */
export function toJson(value: unknown): string;
/** Escapes text for HTML content and double-quoted attributes. */
export function escape(text: string): string;
/** A URL that can't run code: `javascript:` and similar URLs are replaced. */
export function safeUrl(url: string): string;
