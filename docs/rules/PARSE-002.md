# PARSE-002: nested too deeply

**Error**: the page doesn't build until this is fixed.

Elements are nested more than 256 levels deep. Real pages stay far below that; the limit keeps a broken or
generated file from crashing the compiler. Split deep parts into components.

## Fixed

```hypra passes
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  section {
    article { div { p "Hand-written pages stay a few levels deep." } }
  }
}
```
