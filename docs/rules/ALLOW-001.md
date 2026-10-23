# ALLOW-001: allow has no reason

**Error**: the page doesn't build until this is fixed.

`allow` switches a rule off for the next element. Its reason shows up in code review, so a rule is never
switched off by accident or without anyone knowing why.

## Fails

```hypra fails
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  allow("SEO-003")
  h1 "Run faster."
}
```

## Fixed

```hypra passes
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  allow("SEO-003") "the hero splits its headline into two h1"
  h1 "Run faster."
}
```
