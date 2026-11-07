use super::*;

pub(crate) fn live_set(root: &Element) -> Option<&Element> {
    if root.name == "LiveSet" {
        Some(root)
    } else {
        root.get_child("LiveSet")
    }
}

pub(crate) fn live_set_mut(root: &mut Element) -> Option<&mut Element> {
    if root.name == "LiveSet" {
        Some(root)
    } else {
        root.get_mut_child("LiveSet")
    }
}

pub(crate) fn count_descendants(element: &Element, name: &str) -> usize {
    let mut count = usize::from(element.name == name);
    for child in element.children.iter().filter_map(XMLNode::as_element) {
        count += count_descendants(child, name);
    }
    count
}

pub(crate) fn find_descendant<'a>(element: &'a Element, name: &str) -> Option<&'a Element> {
    if element.name == name {
        return Some(element);
    }
    for child in element.children.iter().filter_map(XMLNode::as_element) {
        if let Some(found) = find_descendant(child, name) {
            return Some(found);
        }
    }
    None
}

pub(crate) fn find_descendant_mut<'a>(
    element: &'a mut Element,
    name: &str,
) -> Option<&'a mut Element> {
    if element.name == name {
        return Some(element);
    }
    for child in element
        .children
        .iter_mut()
        .filter_map(XMLNode::as_mut_element)
    {
        if let Some(found) = find_descendant_mut(child, name) {
            return Some(found);
        }
    }
    None
}

pub(crate) fn navigate<'a>(mut element: &'a Element, path: &[usize]) -> Option<&'a Element> {
    for &index in path {
        element = element.children.get(index)?.as_element()?;
    }
    Some(element)
}

pub(crate) fn child_mut<'a>(element: &'a mut Element, name: &str) -> Option<&'a mut Element> {
    element
        .children
        .iter_mut()
        .filter_map(XMLNode::as_mut_element)
        .find(|e| e.name == name)
}

pub(crate) fn ensure_child<'a>(element: &'a mut Element, name: &str) -> &'a mut Element {
    let pos = element
        .children
        .iter()
        .position(|node| node.as_element().is_some_and(|e| e.name == name));
    if let Some(pos) = pos {
        return element.children[pos]
            .as_mut_element()
            .expect("checked element");
    }
    element.children.push(XMLNode::Element(Element::new(name)));
    element
        .children
        .last_mut()
        .and_then(XMLNode::as_mut_element)
        .expect("just pushed")
}

pub(crate) fn set_child_value(parent: &mut Element, name: &str, value: &str) {
    let child = ensure_child(parent, name);
    child.attributes.insert("Value".into(), value.into());
}

pub(crate) fn set_attr(element: &mut Element, name: &str, value: &str) {
    element.attributes.insert(name.into(), value.into());
}

pub(crate) fn attr<'a>(element: &'a Element, name: &str) -> Option<&'a str> {
    element.attributes.get(name).map(String::as_str)
}

pub(crate) fn attr_f64(element: &Element, name: &str) -> Option<f64> {
    attr(element, name).and_then(|v| v.parse().ok())
}

pub(crate) fn child_f64(element: &Element, name: &str) -> f64 {
    element
        .get_child(name)
        .and_then(|e| attr_f64(e, "Value"))
        .unwrap_or(0.0)
}
