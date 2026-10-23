# COMP-009: page expands to too many elements

**Error**: the page doesn't build until this is fixed.

Components are written into the page when it compiles. A component that uses another one ten times, which
uses another one ten times, multiplies: six levels make a million elements, and a page that size would
take long to compile and never load. The limit is 200,000.

## Fails

```hypra fails
component Tile { div "·" }
component Row { Tile Tile Tile Tile Tile Tile Tile Tile Tile Tile }
component Block { Row Row Row Row Row Row Row Row Row Row }
component Wall { Block Block Block Block Block Block Block Block Block Block }
component Floor { Wall Wall Wall Wall Wall Wall Wall Wall Wall Wall }
component House { Floor Floor Floor Floor Floor Floor Floor Floor Floor Floor }
component Street { House House House House House House House House House House }

page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  Street
}
```

## Fixed

```hypra passes
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  props(tiles: List)
  h1 "Shoes"
  for tile in tiles { div "·" }
}
```

Repeating something many times is what `for` is for: a loop runs when the page renders, over the data it gets.
