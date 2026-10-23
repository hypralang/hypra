# ATTR-001: unknown attribute

**Warning**: the page still builds, but this is most likely a mistake.

Hypra knows the attributes of every HTML element. One it doesn't know is most likely a typo, which a
browser would silently ignore, so it's written as-is with a warning. Attributes with a dash, like `data-*`,
`aria-*`, `hx-*` or `x-*`, pass without one.

## Fails

```hypra fails
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  a("/sale", tagret: "_blank") "Sale"
}
```

## Fixed

```hypra passes
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  a("/sale", target: "_blank") "Sale"
  div("popovertarget-id": "cart") "Cart"
}
```

To write an attribute Hypra doesn't know on purpose, quote its name. A near miss of one of Hypra's own
settings, like `titel` on a page, is an error instead: the setting would be lost.
