# FORM-006: unknown form method

**Error**: the page doesn't build until this is fixed.

A form is sent with `get`, `post`, `put`, `patch` or `delete`, or closes a dialog with `dialog`. Any
other method, browsers quietly replace with `get`, which puts the form's values in the address.

## Fails

```hypra fails
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  form("/newsletter", name: "newsletter", method: "send") {
    field("email", type: Email, label: "Email")
    button "Subscribe"
  }
}
```

## Fixed

```hypra passes
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  form("/newsletter", name: "newsletter", method: "post") {
    field("email", type: Email, label: "Email")
    button "Subscribe"
  }
}
```
