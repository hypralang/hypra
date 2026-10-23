# COMP-003: unknown component argument

**Error**: the page doesn't build until this is fixed.

The component has no parameter by that name, so the value would go nowhere. The message lists the
parameters it has.

## Fails

```hypra fails
component ReviewCard(author: Text, text: Text) {
  figure {
    blockquote "{text}"
    figcaption "{author}"
  }
}

page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  ReviewCard(author: "Ana", text: "Light and fast.", stars: 5)
}
```

## Fixed

```hypra passes
component ReviewCard(author: Text, text: Text, stars: Int = 5) {
  figure {
    blockquote "{text}"
    figcaption "{author}, {stars} stars"
  }
}

page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  ReviewCard(author: "Ana", text: "Light and fast.", stars: 5)
}
```

Or add the parameter to the component, as above.
