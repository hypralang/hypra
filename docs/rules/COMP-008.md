# COMP-008: malformed component parameter

**Error**: the page doesn't build until this is fixed.

A parameter is a name with a type, `name: Type`, and maybe a default after `=`. A value after the colon
looks like a default but is read as a type.

## Fails

```hypra fails
component Badge(text: "New") { span(class: "badge") "{text}" }

page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
}
```

## Fixed

```hypra passes
component Badge(text: Text = "New") { span(class: "badge") "{text}" }

page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  Badge
}
```
