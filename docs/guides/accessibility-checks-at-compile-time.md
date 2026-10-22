# Accessibility checks at compile time

By the end of this guide, missing alt text, unlabeled form fields, empty buttons and skipped heading levels stop
your build with an error that says where the problem is and how to fix it, before a page reaches anyone.

## Why check before the page exists

HTML accepts anything. An image without `alt`, an input without a label or a button with only an icon all render
fine, so the mistakes ship silently, and people using screen readers find them first.

Most teams test accessibility on rendered pages: a browser extension, Lighthouse, or
[axe](https://github.com/dequelabs/axe-core) in end-to-end tests. Those tools are good, and they find problems
only a browser can see, like color contrast. But they check the pages and states your tests happen to visit, with
the data your tests happen to use, after the code is written.

## Without Hypra

The usual setup runs axe in Playwright tests:

```console
$ npm install --save-dev @playwright/test @axe-core/playwright
```

```js
// tests/accessibility.spec.js
import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

for (const path of ["/", "/signup", "/shoes/7"]) {
  test(`no accessibility violations on ${path}`, async ({ page }) => {
    await page.goto(path);
    const results = await new AxeBuilder({ page }).analyze();
    expect(results.violations).toEqual([]);
  });
}
```

Keep this. It catches what only a browser can see. Its limits:

- It needs a running app with test data, and it checks only the URLs you list.
- A problem in a component shows up on every page that uses it, but only on pages you test.
- Failures arrive minutes later in CI, as a list of selectors, not as a line in your editor.

## With Hypra

[Hypra](https://hypralang.dev) is a markup language that compiles to HTML. The compiler knows what every element
is, so it checks accessibility while it compiles, on every page and every component, without running anything.
Here's a sign-up page with five common mistakes:

```hypra
page(title: "Sign up", description: "Create an account to save your favourite routes and get one weekly email with new trails.") {
  h1 "Create an account"
  img("/img/trail.jpg")
  h3 "Why sign up?"
  p { "Save routes and get new trails every week. " a("/about") "Click here" }
  form("/signup", name: "signup") {
    field("email", type: Email)
    button
  }
}
```

```console output
$ hypra check signup.hypra
error[A11Y-001]: image has no alt text
 --> signup.hypra:3:7
  |
3 |   img("/img/trail.jpg")
  |       ^^^^^^^^^^^^^^^^
  = help: add alt: "..." or decorative: true

warning[A11Y-006]: h3 follows h1, skipping a level
 --> signup.hypra:4:3
  |
4 |   h3 "Why sign up?"
  |   ^^
  = help: use h2; screen readers navigate by heading levels

warning[A11Y-007]: link text `click here` doesn't say where it goes
 --> signup.hypra:5:65
  |
5 |   p { "Save routes and get new trails every week. " a("/about") "Click here" }
  |                                                                 ^^^^^^^^^^^^
  = help: describe the target, e.g. a("/guide") "Read the setup guide"

error[FORM-002]: `field` needs `label`
 --> signup.hypra:7:5
  |
7 |     field("email", type: Email)
  |     ^^^^^
  = help: add label: "..."

error[A11Y-005]: button has no text
 --> signup.hypra:8:5
  |
8 |     button
  |     ^^^^^^
  = help: button "Send", or aria-label: "..." for an icon button

3 errors, 2 warnings
```

Errors stop the build; warnings don't. Every message has a code, the exact place and the fix. With the
[editor extension](https://hypralang.dev/docs/vscode/), they show up as you type.

The fixed page:

```hypra
page(title: "Sign up", description: "Create an account to save your favourite routes and get one weekly email with new trails.") {
  h1 "Create an account"
  img("/img/trail.jpg", alt: "A runner on a muddy forest trail")
  h2 "Why sign up?"
  p { "Save routes and get new trails every week. " a("/about") "Read how we pick trails" }
  form("/signup", name: "signup") {
    field("email", type: Email, label: "Email")
    button "Sign up"
  }
}
```

What it checks, among others:

| Check | Code |
| --- | --- |
| images need alt text, or `decorative: true`; alt text like "image" or "photo" is a warning | `A11Y-001`, `A11Y-008` |
| links and buttons need text; link text says where it goes, not "click here" | `A11Y-002`, `A11Y-005`, `A11Y-007` |
| form fields need a label, and a form needs a button | `FORM-002`, `FORM-003` |
| headings go down one level at a time, and a page has one `h1` | `A11Y-006`, `SEO-003` |
| videos need captions; autoplaying videos are muted | `A11Y-003`, `A11Y-011` |
| iframes need a title; no positive `tabindex` | `A11Y-009`, `A11Y-010` |

The [error reference](https://hypralang.dev/docs/#errors) lists all of them. A few details that make this work
on real sites:

- **Components are checked once, everywhere.** A component without alt text on its image is an error where the
  component is written, not on each page that uses it.
- **Heading levels are checked as the page renders**, so a component that starts with `h2` is fine after an
  `h1`, and an `h1` inside a loop is caught.
- **Text from data is checked when a page is built with that data**, for example alt text from your database.
- **Exceptions are explicit.** A rule can be turned off for one element, with a reason that shows up in review:

  ```hypra
  allow("A11Y-006") "the card title is styled small on purpose"
  h3 "Trail of the week"
  ```

## Limits

- Hypra can't see what only a browser sees: color contrast, focus order in JavaScript widgets, or whether alt
  text is actually *good*. Keep running axe on rendered pages for those.
- It checks pages written in Hypra. Existing HTML or Blade templates aren't checked; you can move one page at a
  time.
- Hypra is new (version 0.1), and the language may still change before 1.0.

## Next steps

- [Install the compiler](https://hypralang.dev/#install), or try it in Laravel with
  [hypra/laravel](https://hypralang.dev/docs/laravel/).
- The [language reference](https://hypralang.dev/docs/) explains every element and check.
