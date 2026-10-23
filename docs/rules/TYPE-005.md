# TYPE-005: wrong argument for a field type

**Error**: the page doesn't build until this is fixed.

Field types take their limits as arguments: `Int(1..5)` or `Int(min: 0, max: 100)`, `Text(max: 2000)`,
`Choice("s", "m", "l")`. Anything else, like a string where a range belongs, is a mistake.

## Fails

```hypra fails
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  form("/newsletter", name: "review") {
    field("rating", type: Int("1-5"), label: "Rating")
    button "Send"
  }
}
```

## Fixed

```hypra passes
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  form("/newsletter", name: "review") {
    field("rating", type: Int(1..5), label: "Rating")
    button "Send"
  }
}
```
