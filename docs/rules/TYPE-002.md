# TYPE-002: unknown name or field

**Error**: the page doesn't build until this is fixed.

`{name}` in a string reads data, and only names declared in `props`, or the parameters of a component, or a
loop's variable, exist. Usually `props` is missing or the name has a typo.

## Fails

```hypra fails
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  p "Hello {name}, your shoes ship today."
}
```

## Fixed

```hypra passes
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  props(name: Text)
  h1 "Shoes"
  p "Hello {name}, your shoes ship today."
  p "Use {{curly braces}} for literal braces."
}
```

Doubled braces, `{{` and `}}`, are written as plain braces.
