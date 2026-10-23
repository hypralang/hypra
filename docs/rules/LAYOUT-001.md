# LAYOUT-001: unknown layout

**Error**: the page doesn't build until this is fixed.

The page names a layout that isn't defined in the file or in a file it imports. Most often the `use` line
is missing.

```hypra file layouts/shop.hypra
layout Shop(lang: "en", title_suffix: " | Shop") {
  header { a("/") "Shop" }
  main { slot }
}
```

## Fails

```hypra fails
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.", layout: Shop) {
  h1 "Shoes"
}
```

## Fixed

```hypra passes
use "layouts/shop.hypra"

page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.", layout: Shop) {
  h1 "Shoes"
}
```
