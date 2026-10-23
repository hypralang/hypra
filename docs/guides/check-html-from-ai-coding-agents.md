# Check the HTML your AI coding agent writes

By the end of this guide, the pages Claude Code, Codex, Cursor or Copilot write for you are checked after every
change, and the agent fixes what's wrong itself, before you review anything.

## The problem

Coding agents write HTML that looks right. The page renders, the screenshot is fine, the review passes. The
mistakes are the ones you can't see: an image without `alt`, a field without a label, a `div` with a click
handler that a keyboard can't reach, a link that says "Click here", headings that skip a level. People who use a
screen reader or a keyboard find them, and so do search engines and AI agents reading the page.

Here's a menu page of the kind an agent writes when asked for "a simple menu page with an email signup":

```html
<h1>Today's menu</h1>
<img src="/img/loaves.jpg">
<h3>Bread</h3>
<div class="item" onclick="order('rye')">Rye loaf, €4.50</div>
<input type="email" placeholder="Your email">
<a href="/allergens">Click here</a>
```

It has five accessibility problems in six lines. Agents are good at fixing problems once something names them,
so the fix is a check the agent runs after every change, with messages precise enough to act on.

## Without Hypra

[html-validate](https://html-validate.org) checks HTML files offline and fast, which suits agents. Turn on its
accessibility rules:

```console
$ npm install --save-dev html-validate
```

```json
{
  "extends": ["html-validate:recommended", "html-validate:a11y"],
  "rules": { "input-missing-label": "error", "heading-level": "error" }
}
```

On the menu page, as a complete HTML file:

```console
$ npx html-validate menu.html
menu.html
  6:4  error  <img> is missing required "alt" attribute                           wcag/h37
  7:4  error  Heading level can only increase by one, expected <h2> but got <h3>  heading-level
  9:4  error  <input> element does not have a <label>                             input-missing-label

✖ 3 problems (3 errors, 0 warnings)
```

Then tell the agent to run it, in `AGENTS.md` (or `CLAUDE.md` for Claude Code):

```markdown
After changing HTML, run `npx html-validate` on the files you changed and fix every error.
```

This finds three of the five problems. The clickable `div` and the vague link pass, because they're valid HTML.
It also only sees finished HTML: for pages built from templates, it checks what the templates produced for the
data you built them with. For pages that only exist in a browser, [axe](https://github.com/dequelabs/axe-core)
in end-to-end tests covers more, including color contrast, but it's slower and checks only the pages your tests
visit.

## With Hypra

[Hypra](https://hypralang.dev) is a markup language that compiles to HTML, and its compiler refuses pages with
these mistakes. Agents write it from the [language reference](https://hypralang.dev/llms-full.txt) without
having seen it before. The same page, as an agent would first write it, in `menu.hypra`:

```hypra
page(title: "Menu", description: "Today's bread and pastries at Crumb & Co, baked this morning, with prices and allergens.") {
  h1 "Today's menu"
  img("/img/loaves.jpg")
  h3 "Bread"
  div(class: "item", onclick: "order('rye')") "Rye loaf, €4.50"
  input(type: "email", placeholder: "Your email")
  a("/allergens") "Click here"
}
```

```console output
$ hypra check menu.hypra
error[A11Y-001]: image has no alt text
 --> menu.hypra:3:7
  |
3 |   img("/img/loaves.jpg")
  |       ^^^^^^^^^^^^^^^^^
  = help: add alt: "..." or decorative: true
  = docs: https://hypralang.dev/rules/a11y-001/

warning[A11Y-006]: h3 follows h1, skipping a level
 --> menu.hypra:4:3
  |
4 |   h3 "Bread"
  |   ^^
  = help: use h2; screen readers navigate by heading levels
  = docs: https://hypralang.dev/rules/a11y-006/

error[A11Y-014]: `div` with `onclick` isn't a button: a keyboard can't reach it
 --> menu.hypra:5:3
  |
5 |   div(class: "item", onclick: "order('rye')") "Rye loaf, €4.50"
  |   ^^^
  = help: use button(type: "button") or a link; if it has to stay, add role: "button" and tabindex: 0, and handle Enter and Space
  = docs: https://hypralang.dev/rules/a11y-014/

warning[SEC-001]: inline event handler `onclick`
 --> menu.hypra:5:31
  |
5 |   div(class: "item", onclick: "order('rye')") "Rye loaf, €4.50"
  |                               ^^^^^^^^^^^^^^
  = help: put scripts in head { script("/app.js") }
  = docs: https://hypralang.dev/rules/sec-001/

error[A11Y-015]: `input` has no label
 --> menu.hypra:6:3
  |
6 |   input(type: "email", placeholder: "Your email")
  |   ^^^^^
  = help: use field("email", type: Email, label: "Email"), which writes the label; or add aria-label, or a label(for: ...) with the control's id
  = docs: https://hypralang.dev/rules/a11y-015/

warning[A11Y-007]: link text `click here` doesn't say where it goes
 --> menu.hypra:7:19
  |
7 |   a("/allergens") "Click here"
  |                   ^^^^^^^^^^^^
  = help: describe the target, e.g. a("/guide") "Read the setup guide"
  = docs: https://hypralang.dev/rules/a11y-007/

3 errors, 3 warnings
```

All five problems, plus the inline script, each with its line, what to change and a page about the rule. The
errors stop the build, and `hypra check` exits with 1, so an agent knows something is wrong even when it skims
the output. Given these messages, an agent writes this:

```hypra
page(title: "Menu", description: "Today's bread and pastries at Crumb & Co, baked this morning, with prices and allergens.") {
  h1 "Today's menu"
  img("/img/loaves.jpg", alt: "Rye and sourdough loaves on a wooden board")
  h2 "Bread"
  button(type: "button", class: "item", data-order: "rye") "Rye loaf, €4.50"
  form("/signup", name: "signup", description: "Get the menu by email every morning") {
    field("email", type: Email, label: "Email")
    button "Sign up"
  }
  a("/allergens") "Allergens in our bread and pastries"
}
```

Tell agents about it in `AGENTS.md`, the same way as for html-validate:

```markdown
The views in this project are Hypra (.hypra files), not HTML. The language: https://hypralang.dev/llms-full.txt.
After changing .hypra files, run `npx hypra check` on them and fix every error and warning.
```

In Cursor and VS Code, the [extension](https://hypralang.dev/docs/vscode/) shows the same messages in the editor
as the agent writes, and Cursor's agent reads them from there too.

What's different from checking the HTML:

- **The checks know what an element is for.** A clickable `div` and a link that says "Click here" are valid HTML,
  but Hypra reports them, because it checks what a page does, not only whether it parses.
- **Pages are checked before they exist.** `hypra check` reads the page itself, with its layouts and components,
  so a mistake in a component is reported once, whatever data the page shows.
- **The fixes are in the messages.** Each one says what to write instead, in the form the agent is writing.

## Limits

- Only pages written in Hypra are checked this way. For JSX, [eslint-plugin-jsx-a11y](https://github.com/jsx-eslint/eslint-plugin-jsx-a11y)
  does the same job in React projects.
- Some problems only show in a browser, like color contrast or focus order. Keep axe for those.
- Hypra is new and pre-1.0, so the language may still change.

## Next steps

- [Hypra with AI coding agents](https://hypralang.dev/docs/agents/): the full instructions to give an agent.
- [Accessibility checks at compile time](https://hypralang.dev/guides/accessibility-checks-at-compile-time/):
  every check, and how to switch one off for a reason.
