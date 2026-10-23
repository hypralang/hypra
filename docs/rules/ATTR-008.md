# ATTR-008: invalid attribute name

**Error**: the page doesn't build until this is fixed.

A quoted attribute name can be almost anything, like `"@click"` for Alpine.js. But HTML doesn't allow
spaces, quotes, `=`, `>` or `/` in a name: the browser would read it as something else.

## Fails

```hypra fails
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  div("data id": "42") { p "Runner X" }
}
```

## Fixed

```hypra passes
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  div(data-id: "42") { p "Runner X" }
}
```
