# ATTR-002: raw has no HTML

**Error**: the page doesn't build until this is fixed.

`raw` writes HTML exactly as given, for the rare markup Hypra can't express. Without the HTML it has
nothing to write.

## Fails

```hypra fails
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  raw
}
```

## Fixed

```hypra passes
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  raw "<marquee>Sale</marquee>"
}
```
