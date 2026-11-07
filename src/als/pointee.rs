use super::*;

pub(crate) struct PointeeAllocator {
    pub(crate) next: u64,
}

impl PointeeAllocator {
    pub(crate) fn new(live_set: &Element) -> Self {
        let mut max = live_set
            .get_child("NextPointeeId")
            .and_then(|node| attr(node, "Value"))
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(1);
        collect_max_pointee_id(live_set, &mut max);
        Self { next: max + 1 }
    }

    pub(crate) fn allocate(&mut self) -> u64 {
        let id = self.next;
        self.next += 1;
        id
    }
}

pub(crate) fn collect_max_pointee_id(element: &Element, max: &mut u64) {
    if is_pointee_holder(element)
        && let Some(id) = attr(element, "Id").and_then(|value| value.parse::<u64>().ok())
    {
        *max = (*max).max(id);
    }
    for child in element.children.iter().filter_map(XMLNode::as_element) {
        collect_max_pointee_id(child, max);
    }
}

pub(crate) fn is_pointee_holder(element: &Element) -> bool {
    element.name == "Pointee"
        || element.name.ends_with("AutomationTarget")
        || element.name.ends_with("ModulationTarget")
        || element.name.starts_with("ControllerTargets.")
}

pub(crate) fn remap_pointee_ids(element: &mut Element, allocator: &mut PointeeAllocator) {
    let mut remap = HashMap::new();
    collect_pointee_ids(element, allocator, &mut remap);
    rewrite_pointee_ids(element, &remap);
}

pub(crate) fn collect_pointee_ids(
    element: &Element,
    allocator: &mut PointeeAllocator,
    remap: &mut HashMap<u64, u64>,
) {
    if is_pointee_holder(element)
        && let Some(id) = attr(element, "Id").and_then(|value| value.parse::<u64>().ok())
    {
        remap.entry(id).or_insert_with(|| allocator.allocate());
    }
    for child in element.children.iter().filter_map(XMLNode::as_element) {
        collect_pointee_ids(child, allocator, remap);
    }
}

pub(crate) fn rewrite_pointee_ids(element: &mut Element, remap: &HashMap<u64, u64>) {
    if is_pointee_holder(element)
        && let Some(id) = attr(element, "Id").and_then(|value| value.parse::<u64>().ok())
        && let Some(new_id) = remap.get(&id)
    {
        element.attributes.insert("Id".into(), new_id.to_string());
    }
    if element.name == "PointeeId"
        && let Some(id) = attr(element, "Value").and_then(|value| value.parse::<u64>().ok())
        && let Some(new_id) = remap.get(&id)
    {
        element
            .attributes
            .insert("Value".into(), new_id.to_string());
    }
    for child in element
        .children
        .iter_mut()
        .filter_map(XMLNode::as_mut_element)
    {
        rewrite_pointee_ids(child, remap);
    }
}
