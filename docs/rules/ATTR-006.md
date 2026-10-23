# ATTR-006: text on an element that takes none

**Error**: the page doesn't build until this is fixed.

Some elements can't hold text, like a `form`, which holds fields and buttons. The text would end up nowhere,
or somewhere you didn't mean. Put it in its own element.

## Fails

```hypra fails
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  form("/newsletter", name: "newsletter") "Get new shoes first" {
    field("email", type: Email, label: "Email")
    button "Subscribe"
  }
}
```

## Fixed

```hypra passes
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  h2 "Get new shoes first"
  form("/newsletter", name: "newsletter") {
    field("email", type: Email, label: "Email")
    button "Subscribe"
  }
}
```
