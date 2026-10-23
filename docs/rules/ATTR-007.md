# ATTR-007: default outside a component parameter

**Error**: the page doesn't build until this is fixed.

`name = value` gives a component parameter a default. Anywhere else attributes are `name: value`, with a
colon, even though HTML writes `href="/sale"`.

## Fails

```hypra fails
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  a(href = "/sale") "Sale"
}
```

## Fixed

```hypra passes
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  a(href: "/sale") "Sale"
}
```

A default for a component parameter looks like this:

```hypra
component Badge(text: Text = "New") { span(class: "badge") "{text}" }
```
