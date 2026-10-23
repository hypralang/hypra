# ATTR-004: unnamed argument where a name is needed

**Error**: the page doesn't build until this is fixed.

An element's first argument may skip its name only when the element has a main attribute: `a("/sale")` is
the link's `href`, `img("/shoe.jpg")` the image's `src`, `form("/signup")` the form's `action`. Other
elements have no main attribute, so every argument is named.

## Fails

```hypra fails
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  p("intro") "Light shoes for long runs."
}
```

## Fixed

```hypra passes
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  p(class: "intro") "Light shoes for long runs."
}
```
