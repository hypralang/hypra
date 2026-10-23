# TYPE-004: unknown field type

**Error**: the page doesn't build until this is fixed.

A field's type makes its input, its checks and its description for AI agents, so it has to be one Hypra
knows: `String`, `Text`, `Password`, `Email`, `Url`, `Tel`, `Int`, `Number`, `Date`, `Time`, `Choice`,
`Accept` and `File`.

## Fails

```hypra fails
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  form("/newsletter", name: "newsletter") {
    field("email", type: EMail, label: "Email")
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
