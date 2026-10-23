# Hypra for VS Code

Support for [Hypra](https://github.com/hypralang/hypra), the markup language that compiles to HTML for people
and to Markdown and JSON for AI agents. Works in VS Code, Cursor and other editors built on VS Code.

- **Highlighting** for `.hypra` files, including the JavaScript, CSS and JSON inside `script` and `style`.
- **Errors and warnings as you type**, with the rule code, how to fix it, and a link to where the docs explain it.
  Files you import are checked too, as you edit them.
- **Go to Definition** (Ctrl+click or F12) on components, layouts, `use` paths, props and loop variables, also in
  imported files. Ctrl+click on a definition lists where it's used.
- **Find All References** (Shift+F12) and **Rename** (F2), across every `.hypra` file in the folder: renaming a
  component also renames its calls, and renaming a parameter renames the arguments that set it.
- **Hover** over a name to see where it's defined, or over an element or field type to see what it does, with an
  example.
- **Format Document**, the same as `hypra fmt`.
- **Preview** (Ctrl+K V, or the preview button in the editor's title bar): the page next to the editor, as a
  browser shows it, with the project's stylesheets and images. The buttons in the preview's title bar switch to
  the Markdown that AI agents get and to the JSON. It's rendered again whenever a `.hypra` file, a `.json` or a
  `.css` file is saved.

## Install

In Cursor, VSCodium and other editors that install from [Open VSX](https://open-vsx.org/extension/hypralang/hypra),
search for "Hypra" in the Extensions view. In VS Code, download `hypra-<version>.vsix` from the
[latest release](https://github.com/hypralang/hypra/releases/latest), then in the Extensions view choose
**…** → **Install from VSIX…**.

## The preview

A layout or component has no page of its own, so its preview shows a page that uses it, the home page if it
does, with your changes to the layout in it.

The preview uses the data in a JSON file with the page's name next to it, e.g. `product.json` next to
`product.hypra`. Without one, the page is shown with its data left empty. Paths like `/styles.css` are looked up
in the nearest `public` folder above the page, else at the top of the project. Scripts the page loads, like
Tailwind's, run in the preview.

## Settings

Everything except highlighting comes from `hypra lsp`, the language server in the `hypra` compiler. The extension
uses the compiler the project installed, with `npm install hypra` or `php artisan hypra:install`, else `hypra` on
your `PATH`. To use another one, set `hypra.path` to it.

In a folder VS Code doesn't trust yet, `.hypra` files are highlighted but not checked, since checking runs the
compiler, which can be one inside the project. Trusting the folder turns everything on.

| Setting | Default | |
| --- | --- | --- |
| `hypra.path` | `hypra` | the compiler: a command on your `PATH` or a full path, used instead of the project's |
