# ALLOW-003: allow names an unknown rule

**Warning**: the page still builds, but this is most likely a mistake.

The code in `allow` isn't a rule, usually because of a typo, so it switches nothing off. The rule it was
meant for still runs.

## Fails

```hypra fails
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  allow("A11Y-07") "the campaign text is fixed by the brand guide"
  a("/sale") "Sale"
}
```

## Fixed

```hypra passes
page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  h1 "Shoes"
  allow("A11Y-007") "the campaign text is fixed by the brand guide"
  a("/sale") "Click here"
}
```

Every code is listed on the [rules page](https://hypralang.dev/rules/).
