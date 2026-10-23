# TYPE-001: prop missing from the data

**Error**: the page doesn't build until this is fixed.

The page declares a prop that the data doesn't have, so everything that shows it would be empty. It's found
when the page is built or rendered with its data; often the server passes it under another name.

## Fails

```json data
{
  "shoe": { "name": "Runner X" }
}
```

```hypra fails
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  props(product: Product)
  h1 "Shoes"
  p "{product.name}"
}
```

## Fixed

```hypra passes
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  props(shoe: Shoe)
  h1 "Shoes"
  p "{shoe.name}"
}
```
