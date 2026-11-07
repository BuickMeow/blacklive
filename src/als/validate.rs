use super::*;

pub(crate) fn validate_live_set(root: &Element) -> Result<()> {
    validate_sibling_ids(root)?;

    let mut seen = HashSet::new();
    let mut max_pointee = 0u64;
    collect_pointee_ids_for_validation(root, &mut seen, &mut max_pointee)?;

    if let Some(next) = live_set(root)
        .and_then(|live_set| live_set.get_child("NextPointeeId"))
        .and_then(|node| attr(node, "Value"))
        .and_then(|value| value.parse::<u64>().ok())
        && max_pointee >= next
    {
        bail!(
            "Pointee ID 越界：最大 ID {} 不小于 NextPointeeId {}",
            max_pointee,
            next
        );
    }
    Ok(())
}

pub(crate) fn validate_sibling_ids(element: &Element) -> Result<()> {
    let mut seen = HashSet::new();
    for child in element.children.iter().filter_map(XMLNode::as_element) {
        if let Some(id) = attr(child, "Id").and_then(|value| value.parse::<u64>().ok())
            && !seen.insert(id)
        {
            bail!("同一父节点下出现重复 Id：<{} Id=\"{}\">", child.name, id);
        }
    }
    for child in element.children.iter().filter_map(XMLNode::as_element) {
        validate_sibling_ids(child)?;
    }
    Ok(())
}

pub(crate) fn collect_pointee_ids_for_validation(
    element: &Element,
    seen: &mut HashSet<u64>,
    max: &mut u64,
) -> Result<()> {
    if is_pointee_holder(element)
        && let Some(id) = attr(element, "Id").and_then(|value| value.parse::<u64>().ok())
    {
        if !seen.insert(id) {
            bail!("全局 Pointee/AutomationTarget Id 重复：{id}（Live 会拒绝加载）");
        }
        *max = (*max).max(id);
    }
    for child in element.children.iter().filter_map(XMLNode::as_element) {
        collect_pointee_ids_for_validation(child, seen, max)?;
    }
    Ok(())
}
