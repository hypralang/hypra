# COMP-010: content without a slot

**Error**: the page doesn't build until this is fixed.

Content inside a component call goes where the component's body says `slot`. A component without a `slot`
has nowhere to put it, and it would disappear.

## Fails

```hypra fails
component Badge(text: Text) { span(class: "badge") "{text}" }

page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  Badge(text: "New") { p "Just in" }
}
```

## Fixed

```hypra passes
component Badge(text: Text) {
  span(class: "badge") "{text}"
  slot
}

page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  Badge(text: "New") { p "Just in" }
}
```
