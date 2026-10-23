# IMPORT-001: imported file can't be read

**Error**: the page doesn't build until this is fixed.

The file in a `use` line doesn't exist, or can't be read. Paths are relative to the file that imports
them, so `use "components/review-card.hypra"` in `pages/shoe.hypra` means `pages/components/review-card.hypra`.

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
use "components/reveiw-card.hypra"

page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"

}
```

## Fixed

```hypra passes
use "components/review-card.hypra"

page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"

}
```
