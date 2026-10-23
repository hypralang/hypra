# FORM-002: field has no label

**Error**: the page doesn't build until this is fixed.

A label tells everyone what to type, screen readers announce it, and clicking it puts the cursor in the
field. Placeholder text isn't a label: it disappears as soon as someone types.
([WCAG 3.3.2](https://www.w3.org/WAI/WCAG22/Understanding/labels-or-instructions))

## Fails

```hypra fails
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  form("/newsletter", name: "newsletter") {
    field("email", type: Email)
    button "Subscribe"
  }
}
```

## Fixed

```hypra passes
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  form("/newsletter", name: "newsletter") {
    field("email", type: Email, label: "Email")
    button "Subscribe"
  }
}
```
