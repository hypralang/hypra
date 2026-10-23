# LAYOUT-004: layout name isn't capitalized

**Error**: the page doesn't build until this is fixed.

Layouts, like components, start with a capital letter, so `layout: Shop` is clearly a name and not data.

## Fails

```hypra fails
layout shop(lang: "en", title_suffix: " | Shop") {
  header { a("/") "Shop" }
  main { slot }
}

page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
}
```

## Fixed

```hypra passes
layout Shop(lang: "en", title_suffix: " | Shop") {
  header { a("/") "Shop" }
  main { slot }
}

page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.", layout: Shop) {
  h1 "Shoes"
}
```
