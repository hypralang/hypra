# LAYOUT-002: layout has no slot

**Error**: the page doesn't build until this is fixed.

A layout is the frame around a page, and `slot` is where the page goes. Without it, the page's content
would disappear, which is why the example also loses its `h1`.

## Fails

```hypra fails SEO-004
layout Shop {
  header { a("/") "Shop" }
  footer { p "Free returns within 30 days." }
}

page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.", layout: Shop) {
  h1 "Shoes"
}
```

## Fixed

```hypra passes
layout Shop {
  header { a("/") "Shop" }
  main { slot }
  footer { p "Free returns within 30 days." }
}

page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.", layout: Shop) {
  h1 "Shoes"
}
```
