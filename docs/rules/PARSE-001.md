# PARSE-001: syntax error

**Error**: the page doesn't build until this is fixed.

The file doesn't have the shape Hypra expects at this point. The message says what was expected and what
was found; the usual causes are a missing `)` or `}`, or an attribute without its value.

## Fails

```hypra fails
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  img("/img/runner-x.jpg", alt: )
}
```

## Fixed

```hypra passes
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  img("/img/runner-x.jpg", alt: "Runner X in blue, side view")
}
```
