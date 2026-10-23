# FORM-001: field has no name or type

**Error**: the page doesn't build until this is fixed.

A field's name is the key its value is sent under. Its type, like `Email` or `Int(1..5)`, makes the right
input, checks the value in the browser and on the server, and describes the field to AI agents.

## Fails

```hypra fails
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  form("/newsletter", name: "newsletter") {
    field(type: Email, label: "Email")
    field("size", label: "Shoe size")
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
    field("size", type: Int(36..48), label: "Shoe size")
    button "Subscribe"
  }
}
```
