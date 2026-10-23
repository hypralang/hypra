# LAYOUT-005: layout inside itself

**Error**: the page doesn't build until this is fixed.

A layout can sit inside another one, like a shop's frame inside the site's, but never inside itself: the
frames would go on forever.

## Fails

```hypra fails
layout Shop(layout: Shop) {
  main { slot }
}

page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.", layout: Shop) {
  h1 "Shoes"
}
```

## Fixed

```hypra passes
layout Base(lang: "en") {
  main { slot }
}

layout Shop(layout: Base) {
  nav(aria-label: "Shop") { a("/shoes") "All shoes" }
  slot
}

page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.", layout: Shop) {
  h1 "Shoes"
}
```
