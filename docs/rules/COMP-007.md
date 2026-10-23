# COMP-007: component calls itself

**Error**: the page doesn't build until this is fixed.

Components are written into the page when it compiles, before any data is there. A component that uses
itself would never stop, so a menu with submenus is written as levels.

## Fails

```hypra fails
component Menu(items: List) {
  ul {
    for item in items {
      li {
        a(item.url) "{item.title}"
        Menu(items: item.children)
      }
    }
  }
}

page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  props(menu: List)
  h1 "Shoes"
  Menu(items: menu)
}
```

## Fixed

```hypra passes
component Menu(items: List) {
  ul {
    for item in items {
      li {
        a(item.url) "{item.title}"
        ul {
          for child in item.children { li { a(child.url) "{child.title}" } }
        }
      }
    }
  }
}

page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  props(menu: List)
  h1 "Shoes"
  Menu(items: menu)
}
```
