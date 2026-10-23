# ATTR-003: wrong kind of value

**Error**: the page doesn't build until this is fixed.

Hypra's own settings take one kind of value: `priority` and `decorative` are `true` or `false`, `width` is a
number. A string like `"yes"` would mean something different to every reader, so it isn't accepted.

## Fails

```hypra fails
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  img("/img/runner-x.jpg", alt: "Runner X in blue, side view", priority: "yes")
}
```

## Fixed

```hypra passes
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  img("/img/runner-x.jpg", alt: "Runner X in blue, side view", priority: true)
}
```

Settings like `priority` are also fixed when the page compiles, so they can't come from data.
