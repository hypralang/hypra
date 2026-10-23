# COMP-001: unknown component

**Error**: the page doesn't build until this is fixed.

A capitalized name is a component, and this one isn't defined in the file or in a file it imports. Most
often the `use` line is missing.

```hypra file components/review-card.hypra
component ReviewCard(author: Text, text: Text) {
  figure {
    blockquote "{text}"
    figcaption "{author}"
  }
}
```

## Fails

```hypra fails
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  ReviewCard(author: "Ana", text: "Light and fast.")
}
```

## Fixed

```hypra passes
use "components/review-card.hypra"

page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  ReviewCard(author: "Ana", text: "Light and fast.")
}
```
