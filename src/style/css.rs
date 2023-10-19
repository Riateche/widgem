use std::collections::HashMap;

use itertools::Itertools;
use lightningcss::{
    properties::{
        custom::{CustomPropertyName, TokenOrValue},
        Property,
    },
    rules::CssRule,
    selector::{Component, PseudoClass, Selector},
    stylesheet::StyleSheet,
};
use log::warn;

pub fn replace_vars(style_sheet: &mut StyleSheet) {
    //let mut style_sheet: StyleSheet<'static, 'static> = style_sheet.into_owned();
    let mut vars = HashMap::new();
    for rule in &style_sheet.rules.0 {
        if let CssRule::Style(rule) = rule {
            // println!("selectors: {:?}", rule.selectors);
            for selector in &rule.selectors.0 {
                print_selector(selector);
                if is_root(selector) {
                    println!("found root!");
                    for (property, _) in rule.declarations.iter() {
                        //println!("root declaration: {declaration:?}");
                        if let Property::Custom(property) = property {
                            if let CustomPropertyName::Custom(name) = &property.name {
                                vars.insert(name.as_ref().to_string(), property.value.clone());
                            }
                        }
                    }
                }
                //print_selector(selector);
            }
        }
    }
    for rule in &mut style_sheet.rules.0 {
        if let CssRule::Style(rule) = rule {
            for property in rule.declarations.iter_mut() {
                if let Property::Unparsed(property) = property {
                    let mut new_tokens = Vec::new();
                    for token in &property.value.0 {
                        if let TokenOrValue::Var(variable) = token {
                            if let Some(value) = vars.get(variable.name.ident.as_ref()) {
                                println!("substitute!");
                                // TODO: use substitute_variables
                                new_tokens.extend(value.0.clone());
                                continue;
                            }
                        }
                        new_tokens.push(token.clone());
                    }
                    property.value.0 = new_tokens;
                }
            }
        }
    }

    println!("vars: {vars:#?}");
}

fn print_selector(selector: &Selector) {
    println!("selector: {:?}", selector);
    let mut iter = selector.iter();
    loop {
        for x in &mut iter {
            println!("selector item: {:?}", x);
            print_component(x);
            if matches!(x, Component::Root) {
                println!("found root!");
            }
            if let Component::Negation(inner) = x {
                println!("found not! inner:");
                print_selector(&inner[0]);
                println!("inner end");
            }
        }
        if let Some(seq) = iter.next_sequence() {
            println!("seq: {seq:?}");
        } else {
            println!("no seq");
            break;
        }
    }
}

fn print_component(component: &Component) {
    match component {
        Component::Combinator(_) => println!("Combinator"),
        Component::ExplicitAnyNamespace => println!("ExplicitAnyNamespace"),
        Component::ExplicitNoNamespace => println!("ExplicitNoNamespace"),
        Component::DefaultNamespace(_) => println!("DefaultNamespace"),
        Component::Namespace(..) => println!("Namespace"),
        Component::ExplicitUniversalType => println!("ExplicitUniversalType"),
        Component::LocalName(_) => println!("LocalName"),
        Component::ID(_) => println!("ID"),
        Component::Class(_) => println!("Class"),
        Component::AttributeInNoNamespaceExists { .. } => println!("AttributeInNoNamespaceExists"),
        Component::AttributeInNoNamespace { .. } => println!("AttributeInNoNamespace"),
        Component::AttributeOther(_) => println!("AttributeOther"),
        Component::Negation(_) => println!("Negation"),
        Component::Root => println!("Root"),
        Component::Empty => println!("Empty"),
        Component::Scope => println!("Scope"),
        Component::Nth(_) => println!("Nth"),
        Component::NthOf(_) => println!("NthOf"),
        Component::NonTSPseudoClass(x) => {
            println!("NonTSPseudoClass");
            if let PseudoClass::Custom { name } = x {
                println!("name = {name:?}");
            }
        }
        Component::Slotted(_) => println!("Slotted"),
        Component::Part(_) => println!("Part"),
        Component::Host(_) => println!("Host"),
        Component::Where(_) => println!("Where"),
        Component::Is(_) => println!("Is"),
        Component::Any(..) => println!("Any"),
        Component::Has(_) => println!("Has"),
        Component::PseudoElement(_) => println!("PseudoElement"),
        Component::Nesting => println!("Nesting"),
    }
}

pub fn selector_items<'i, 'a>(selector: &'a Selector<'i>) -> Option<Vec<&'a Component<'i>>> {
    let mut iter = selector.iter();
    let components = (&mut iter).collect_vec();
    if iter.next_sequence().is_some() {
        warn!("nesting in CSS selectors is not supported (selector: {selector:?})");
        // We don't support nesting in selectors.
        return None;
    }
    Some(components)
}

pub fn is_root(selector: &Selector) -> bool {
    selector_items(selector).map_or(false, |items| {
        items.len() == 1 && matches!(items[0], Component::Root)
    })
}
