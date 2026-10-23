# COMP-002: missing component argument

**Error**: the page doesn't build until this is fixed.

A component's parameters without a default have to be given every time it's used. Otherwise its text
would be empty.

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
  ReviewCard(author: "Ana")
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

A parameter that may be left out gets a default: `text: Text = ""`.
