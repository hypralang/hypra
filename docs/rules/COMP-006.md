# COMP-006: unnamed component argument

**Error**: the page doesn't build until this is fixed.

Component arguments are always named, so a call reads the same however the component orders its
parameters, and adding one later doesn't shift the others.

## Fails

```hypra fails
component Badge(text: Text = "New") { span(class: "badge") "{text}" }

page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  Badge("Sale")
}
```

## Fixed

```hypra passes
component Badge(text: Text = "New") { span(class: "badge") "{text}" }

page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  Badge(text: "Sale")
}
```
