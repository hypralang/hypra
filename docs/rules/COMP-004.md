# COMP-004: component defined twice

**Error**: the page doesn't build until this is fixed.

Two components with the same name, in one file or across the files it imports, would make every use of the
name ambiguous.

## Fails

```hypra fails
component ReviewCard(author: Text, text: Text) {
  figure {
    blockquote "{text}"
    figcaption "{author}"
  }
}

component ReviewCard(author: Text, text: Text) {
  article {
    blockquote "{text}"
    figcaption "{author}"
  }
}

page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  ReviewCard(author: "Ana", text: "Light and fast.")
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
