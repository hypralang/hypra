# Add WebMCP tools to your website

By the end of this guide, the forms and buttons on your pages are also tools that AI agents can call, with a
name, a description and a typed list of parameters, generated from the page itself.

## What WebMCP is

Today an AI agent that wants to use a website has to look at the page and guess: which button adds to the cart,
which field is the email, what values are allowed. WebMCP is a proposal from the W3C Web Machine Learning
community group to let a page say it directly: "this page offers these tools, with these parameters". It borrows
the idea of tools from the Model Context Protocol (MCP), which AI assistants already use to call APIs.

WebMCP has two parts:

- a **declarative** one: attributes on an ordinary HTML form, `toolname` and `tooldescription`, from which the
  browser builds the tool and its parameters;
- an **imperative** one: a JavaScript API for tools that aren't forms.

It's experimental. Chrome is testing it, the details are still changing, and other browsers haven't committed to
it. The declarative form is the safe start: browsers that don't know the attributes ignore them, so the forms work
as they always did.

## Without Hypra

You can add the attributes by hand to any form:

```html
<form method="post" action="/checkout" toolname="checkout"
      tooldescription="Pay for the cart and choose how it's delivered">
  <label>Delivery
    <select name="delivery" required>
      <option value="standard">Standard, 3–5 days</option>
      <option value="express">Express, next day</option>
    </select>
  </label>
  <label>Email for the receipt <input type="email" name="email" required></label>
  <button type="submit">Pay now</button>
</form>
```

That's a good start. What it leaves to you:

- The tool's parameters are only as good as the form's markup: a field without `name`, `type` or `required`
  becomes a vague parameter, and nothing tells you.
- Buttons that do one thing, like "remove from cart", need to be wrapped in forms to become tools at all, and a
  list of ten of them becomes ten tools with the same name.
- Agents that don't run a browser, like a crawler or an assistant reading your page as text, don't see the
  attributes.

## With Hypra

[Hypra](https://hypralang.dev) is a markup language that compiles to HTML. Its forms and actions are typed, so
every one of them is a tool, in every output the compiler writes. A cart page:

```hypra
page(title: "Your cart", description: "The shoes in your cart, ready to check out, with free shipping on orders over €100.") {
  props(cart: Cart)
  h1 "Your cart"
  for item in cart.items {
    article {
      h2 "{item.name}"
      p "Size {item.size}"
      action("cart.remove", id: item.id, confirm: "Remove {item.name}?") "Remove from cart"
    }
  }
  form("/checkout", name: "checkout", description: "Pay for the cart and choose how it's delivered") {
    field("delivery", type: Choice(standard: "Standard, 3–5 days", express: "Express, next day"), label: "Delivery")
    field("email", type: Email, label: "Email for the receipt")
    field("note", type: Text(max: 200), label: "Note for the courier", optional: true)
    button "Pay now"
  }
}
```

with this data:

```json
{
  "cart": {
    "items": [
      { "id": 11, "name": "Trail Runner GTX", "size": 42 },
      { "id": 12, "name": "Road Racer", "size": 43 }
    ]
  }
}
```

### For browsers: WebMCP attributes

The checkout form becomes a normal HTML form with the WebMCP attributes, labels on every field and the
constraints from the field types:

```html output
<form method="post" action="/checkout" toolname="checkout" tooldescription="Pay for the cart and choose how it's delivered">
  <label>Delivery <select name="delivery" required><option value="">Delivery</option><option value="standard">Standard, 3–5 days</option><option value="express">Express, next day</option></select></label>
  <label>Email for the receipt <input type="email" name="email" required></label>
  <label>Note for the courier <textarea name="note" maxlength="200"></textarea></label>
  <button>Pay now</button>
</form>
```

Each **Remove from cart** button becomes a small form of its own with `toolname="cart.remove"`, and posts to
`/_a/cart.remove`.

### For agents: the tools as JSON

The compiler also writes the page's tools as a JSON file, `cart.webmcp.json`, with a JSON Schema for each tool's
parameters. This file is Hypra's own, not part of the WebMCP proposal; it's for agents and tests that work
without a browser.

```json output
[
  {
    "name": "cart.remove",
    "description": "Remove from cart",
    "inputSchema": {
      "type": "object",
      "properties": {
        "id": {
          "type": "integer",
          "enum": [
            11,
            12
          ]
        }
      },
      "required": [
        "id"
      ]
    }
  },
  {
    "name": "checkout",
    "description": "Pay for the cart and choose how it's delivered",
    "inputSchema": {
      "type": "object",
      "properties": {
        "delivery": {
          "type": "string",
          "enum": [
            "standard",
            "express"
          ]
        },
        "email": {
          "type": "string",
          "format": "email"
        },
        "note": {
          "type": "string",
          "maxLength": 200
        }
      },
      "required": [
        "delivery",
        "email"
      ]
    }
  }
]
```

The two **Remove from cart** buttons became *one* tool, `cart.remove`, whose `id` must be one of the items in the
cart. A field with `optional: true` isn't required; a `Choice` becomes an `enum`; `Text(max: 200)` becomes
`maxLength`.

### For LLMs: the actions in Markdown

The page's Markdown version, for agents that read the page as text, ends with the same tools:

```markdown output
## Actions

- `cart.remove(id: 11|12)`: Remove from cart → POST /_a/cart.remove
- `checkout(delivery: standard|express, email: email, note?: string ..200)`: Pay for the cart and choose how it's delivered → POST /checkout
```

With the [Laravel package](https://hypralang.dev/docs/laravel/), that Markdown is served from the page's own
URL to clients that ask for it, and `action("cart.remove", ...)` posts to a route you connect with
`Route::hypra('cart.remove', ...)`. CSRF tokens are added to every form posting to your site.

## Limits

- WebMCP itself is experimental and may change; Hypra will follow it.
- The tools describe what a page offers. Your server still has to check every request, as with any form: an
  agent can send anything a browser can.
- Hypra is new (version 0.1), and the language may still change before 1.0.

## Next steps

- [Forms](https://hypralang.dev/docs/#forms) and [actions](https://hypralang.dev/docs/#actions) in the language
  reference.
- [Serve Markdown to LLMs from Laravel](https://hypralang.dev/guides/markdown-for-llms-in-laravel/), the same
  page for agents that read text.
