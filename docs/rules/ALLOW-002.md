# ALLOW-002: allow has no rule code

**Error**: the page doesn't build until this is fixed.

`allow` switches off one rule, named by its code, for the next element. Without the code it switches off
nothing, so the rule it was meant for still runs. The code is in every message: `error[SEO-003]`.

## Fails

```hypra fails SEO-003
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  allow "the hero splits its headline into two h1"
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
