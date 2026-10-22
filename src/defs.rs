//! Collects components and layouts from every file in a project, so the checker and lowering
//! can resolve `Card(...)` calls and `page(layout: Shop)`.

use crate::diag::Diagnostic;
use crate::html;
use crate::parser::{Arg, Def, Element, Item, Value, any_element, is_name};
use std::collections::HashMap;

#[derive(Default)]
pub struct Defs<'a> {
    pub components: HashMap<String, &'a Def>,
    pub layouts: HashMap<String, &'a Def>,
}

pub fn collect(modules: &[Vec<Item>]) -> (Defs<'_>, Vec<Diagnostic>) {
    let mut defs = Defs::default();
    let mut diags = vec![];
    for item in modules.iter().flatten() {
        let (map, code, kind, def) = match item {
            Item::Component(def) => (&mut defs.components, "COMP-004", "component", def),
            Item::Layout(def) => (&mut defs.layouts, "LAYOUT-003", "layout", def),
            _ => continue,
        };
        if map.insert(def.name.clone(), def).is_some() {
            diags.push(Diagnostic::error(
                code,
                format!("{kind} `{}` is defined twice", def.name),
                def.name_pos,
                def.name.len(),
            ));
        }
    }
    (defs, diags)
}

/// A parameter is either `name: Type` or just `name`.
pub fn param_name(param: &Arg) -> Option<&str> {
    param.name().or_else(|| param.value.as_path())
}

/// The one element a component renders, which takes the HTML attributes a call adds: `Card(class: "mt-4")`.
/// Only a plain HTML element qualifies; Hypra's own elements like `img` or `form` build their tags themselves.
pub fn forward_root(def: &Def) -> Option<&Element> {
    let mut rendered = def.body.iter().filter(|item| !matches!(item, Item::Element(e) if e.name == "allow"));
    match (rendered.next(), rendered.next()) {
        (Some(Item::Element(root)), None)
            if !is_name(&root.name)
                && !matches!(root.name.as_str(), "img" | "video" | "action" | "form" | "field" | "slot") =>
        {
            Some(root)
        }
        _ => None,
    }
}

/// An argument name that's an HTML attribute rather than a component parameter.
pub fn is_html_attribute(name: &str) -> bool {
    html::ATTRIBUTES.contains(&name) || name.contains('-') || name.contains(':') || name.starts_with('@')
}

/// The layout a page or layout names, then the layouts around it, innermost first: `Docs`, then the `Site` it
/// sits in. A loop of layouts stops where it repeats; the checker reports it.
pub fn layout_chain<'a>(defs: &Defs<'a>, first: Option<&Value>) -> Vec<&'a Def> {
    let mut chain: Vec<&'a Def> = vec![];
    let mut next = first.and_then(Value::as_name).and_then(|name| defs.layouts.get(name).copied());
    while let Some(layout) = next {
        if chain.iter().any(|l| std::ptr::eq(*l, layout)) {
            break;
        }
        chain.push(layout);
        next = setting(layout, "layout").and_then(Value::as_name).and_then(|name| defs.layouts.get(name).copied());
    }
    chain
}

/// A layout's own setting, like `title_suffix`.
pub fn setting<'a>(layout: &'a Def, key: &str) -> Option<&'a Value> {
    layout.params.iter().find(|p| p.name() == Some(key)).map(|p| &p.value)
}

/// A setting from the innermost layout that has it.
pub fn chain_setting<'a>(chain: &[&'a Def], key: &str) -> Option<&'a Value> {
    chain.iter().find_map(|layout| setting(layout, key))
}

/// Whether a component or layout has the slot with this name, or the default slot for `None`.
pub fn contains_slot(items: &[Item], name: Option<&str>) -> bool {
    any_element(items, &|e| e.name == "slot" && slot_name(e) == name)
}

/// The name of `slot("actions")`; the default slot has none.
pub fn slot_name(slot: &Element) -> Option<&str> {
    slot.main_or("name").and_then(|v| v.as_str())
}

/// What a call gives a `slot`: the content of the named slot, or for the default slot everything not given to a
/// named one. `None` when the call gives a named slot nothing, so the slot shows its own content.
pub fn given_content<'a>(call: &'a [Item], slot: &Element) -> Option<Vec<&'a Item>> {
    let items = match slot_name(slot) {
        Some(name) => &call.iter().filter_map(named_content).find(|(n, _)| *n == name)?.1.children[..],
        None => call,
    };
    Some(items.iter().filter(|item| named_content(item).is_none()).collect())
}

/// Content given to a named slot in a component call: `Card { slot("actions") { ... } }`.
pub fn named_content(item: &Item) -> Option<(&str, &Element)> {
    match item {
        Item::Element(e) if e.name == "slot" => slot_name(e).map(|name| (name, e)),
        _ => None,
    }
}
