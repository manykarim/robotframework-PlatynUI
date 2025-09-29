use std::collections::{BTreeMap, VecDeque};

use serde_json::{json, Value};
use uiautomation::core::{UIAutomation, UIElement};
use uiautomation::types::TreeScope;

use super::BackendError;
use crate::config::{AppConfig, Win32Config, Win32Root};
use crate::model::UiNode;

pub(super) fn load_win32_tree(config: &AppConfig) -> Result<UiNode, BackendError> {
    let automation = UIAutomation::new()?;
    let target = resolve_root(&automation, &config.win32)?;
    build_subtree(&automation, &target)
}

fn resolve_root(
    automation: &UIAutomation,
    options: &Win32Config,
) -> Result<UIElement, BackendError> {
    let base = match options.root {
        Win32Root::Desktop => automation.get_root_element()?,
        Win32Root::Focused => automation.get_focused_element()?,
    };

    let has_selectors = options.process_id.is_some() || options.window_title.is_some();
    if !has_selectors {
        return Ok(base);
    }

    if matches_selectors(&base, options)? {
        return Ok(base);
    }

    let mut queue: VecDeque<(UIElement, usize)> = VecDeque::new();
    let condition = automation.create_true_condition()?;

    if options.top_level_only {
        for child in base.find_all(TreeScope::Children, &condition)? {
            queue.push_back((child, 1));
        }
    } else {
        queue.push_back((base.clone(), 0));
    }

    while let Some((element, depth)) = queue.pop_front() {
        if matches_selectors(&element, options)? {
            return Ok(element);
        }

        if options.top_level_only && depth >= 1 {
            continue;
        }

        for child in element.find_all(TreeScope::Children, &condition)? {
            queue.push_back((child, depth + 1));
        }
    }

    Err(BackendError::WindowsTargetNotFound {
        selectors: describe_selectors(options),
    })
}

fn matches_selectors(element: &UIElement, options: &Win32Config) -> Result<bool, BackendError> {
    if let Some(pid) = options.process_id {
        match element.get_process_id() {
            Ok(actual) if actual == pid => {}
            Ok(_) => return Ok(false),
            Err(_) => return Ok(false),
        }
    }

    if let Some(target) = &options.window_title {
        let Ok(name) = element.get_name() else {
            return Ok(false);
        };
        if !name.to_lowercase().contains(target) {
            return Ok(false);
        }
    }

    Ok(true)
}

fn describe_selectors(options: &Win32Config) -> String {
    let mut parts = Vec::new();
    if let Some(pid) = options.process_id {
        parts.push(format!("process_id={pid}"));
    }
    if let Some(title) = &options.window_title {
        parts.push(format!("window_title contains '{title}'"));
    }
    if parts.is_empty() {
        "<none>".to_string()
    } else {
        parts.join(", ")
    }
}

fn build_subtree(automation: &UIAutomation, element: &UIElement) -> Result<UiNode, BackendError> {
    let name = element.get_name().unwrap_or_default();
    let role = element
        .get_localized_control_type()
        .ok()
        .filter(|value| !value.is_empty());

    let mut attributes: BTreeMap<String, Value> = BTreeMap::new();

    if let Ok(control_type) = element.get_control_type() {
        attributes.insert(
            "ControlType".to_string(),
            Value::String(format!("{control_type:?}")),
        );
    }

    if let Ok(auto_id) = element.get_automation_id() {
        if !auto_id.is_empty() {
            attributes.insert("AutomationId".into(), Value::String(auto_id));
        }
    }

    if let Ok(class_name) = element.get_classname() {
        if !class_name.is_empty() {
            attributes.insert("ClassName".into(), Value::String(class_name));
        }
    }

    if let Ok(framework) = element.get_framework_id() {
        if !framework.is_empty() {
            attributes.insert("FrameworkId".into(), Value::String(framework));
        }
    }

    if let Ok(help_text) = element.get_help_text() {
        if !help_text.is_empty() {
            attributes.insert("HelpText".into(), Value::String(help_text));
        }
    }

    if let Ok(process_id) = element.get_process_id() {
        attributes.insert("ProcessId".into(), json!(process_id));
    }

    if let Ok(handle) = element.get_native_window_handle() {
        if !handle.is_invalid() {
            attributes.insert(
                "NativeWindowHandle".into(),
                Value::String(format!("{handle:?}")),
            );
        }
    }

    if let Ok(rect) = element.get_bounding_rectangle() {
        attributes.insert(
            "BoundingRectangle".into(),
            json!({
                "left": rect.get_left(),
                "top": rect.get_top(),
                "right": rect.get_right(),
                "bottom": rect.get_bottom(),
                "width": rect.get_width(),
                "height": rect.get_height(),
            }),
        );
    }

    if let Ok(is_enabled) = element.is_enabled() {
        attributes.insert("IsEnabled".into(), json!(is_enabled));
    }

    if let Ok(is_offscreen) = element.is_offscreen() {
        attributes.insert("IsOffscreen".into(), json!(is_offscreen));
    }

    let condition = automation.create_true_condition()?;
    let mut children = Vec::new();
    for child in element.find_all(TreeScope::Children, &condition)? {
        children.push(build_subtree(automation, &child)?);
    }

    Ok(UiNode {
        name,
        role,
        attributes,
        children,
    })
}
