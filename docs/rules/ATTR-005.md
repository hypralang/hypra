# ATTR-005: attribute set twice

**Error**: the page doesn't build until this is fixed.

Browsers keep the first of two equal attributes and drop the other without a word, so one of them never
takes effect.

## Fails

```hypra fails
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  div(class: "card", class: "featured") { p "Runner X" }
}
```

## Fixed

```hypra passes
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  div(class: "card featured") { p "Runner X" }
}
```
