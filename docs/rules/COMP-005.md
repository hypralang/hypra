# COMP-005: component name isn't capitalized

**Error**: the page doesn't build until this is fixed.

A capital letter is how Hypra, and anyone reading the page, tells a component from an HTML element:
`ReviewCard` is yours, `figure` is HTML's.

## Fails

```hypra fails
component reviewCard(author: Text, text: Text) {
  figure {
    blockquote "{text}"
    figcaption "{author}"
  }
}

page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"

}
```

## Fixed

```hypra passes
component ReviewCard(author: Text, text: Text) {
  figure {
    blockquote "{text}"
    figcaption "{author}"
  }
}

page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  ReviewCard(author: "Ana", text: "Light and fast.")
}
```
