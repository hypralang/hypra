# TYPE-003: value isn't a list

**Error**: the page doesn't build until this is fixed.

`for` goes through a list, and the data has something else there, like a string. It's found when the page
is built or rendered with its data.

## Fails

```json data
{
  "shoe": { "sizes": "38-47" }
}
```

```hypra fails
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  props(shoe: Shoe)
  h1 "Shoes"
  ul { for size in shoe.sizes { li "{size}" } }
}
```

## Fixed

```json data
{
  "shoe": { "sizes": [38, 39, 40] }
}
```

```hypra passes
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  props(shoe: Shoe)
  h1 "Shoes"
  ul { for size in shoe.sizes { li "{size}" } }
}
```
