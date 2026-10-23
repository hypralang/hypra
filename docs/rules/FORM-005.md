# FORM-005: field outside a form

**Error**: the page doesn't build until this is fixed.

A field belongs to the form it's in: the form sends its value, and the form's tool for AI agents lists it.
Fields are known when the page compiles, so they can't be inside `for` or `if` in the form either.

## Fails

```hypra fails
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  field("email", type: Email, label: "Email")
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

A choice that comes from data is a `Choice` field, whose options are data; a field that only some people fill in is `optional: true`.
