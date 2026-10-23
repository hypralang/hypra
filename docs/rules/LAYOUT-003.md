# LAYOUT-003: layout defined twice

**Error**: the page doesn't build until this is fixed.

Two layouts with the same name, in one file or across the files it imports, would make `layout: Shop`
ambiguous.

## Fails

```hypra fails
layout Shop(lang: "en", title_suffix: " | Shop") {
  header { a("/") "Shop" }
  main { slot }
}

layout Shop(lang: "en", title_suffix: " | Shop") {
  header { a("/") "Shop" }
  div { slot }
}

page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.", layout: Shop) {
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
