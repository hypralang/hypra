# FORM-004: form with fields has no name

**Warning**: the page still builds, but this is most likely a mistake.

A form's name is the name AI agents call it by: it becomes the form's WebMCP tool, and the action listed in
the page's Markdown and JSON.

## Fails

```hypra fails
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  form("/newsletter") {
    field("email", type: Email, label: "Email")
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
