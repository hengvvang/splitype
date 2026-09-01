//! Settings declaration vocabulary.
//!
//! Plugins declare their settings schema in their manifests as pure data;
//! the settings UI renders entirely from these declarations and never
//! imports plugin code. The platform knows nothing about what any setting
//! configures — only its key, control kind, default, and display metadata.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The control kind of one settings key, plus kind-specific presentation
/// parameters. The settings host renders each kind with the matching
/// control.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingKind {
    /// An on/off toggle; the value is a JSON boolean.
    Bool,
    /// A numeric stepper with inline editing; the value is a JSON number.
    Number,
    /// A single-line text input; the value is a JSON string.
    String,
    /// A dropdown over the declared options; the value is a JSON string
    /// matching one option's value.
    Enum,
    /// A searchable font-family picker; the value is a JSON string font
    /// name. The host resolves the available fonts at render time.
    Font,
    /// A theme picker; the value is a JSON string theme id. The host
    /// resolves the available themes at render time and applies the
    /// selection live.
    Theme,
    /// A language picker; the value is a JSON string language id. The host
    /// resolves the available languages at render time and applies the
    /// selection live.
    Language,
}

/// One selectable option of an [`SettingKind::Enum`] setting.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SettingOption {
    /// The stored value of the option.
    pub value: String,
    /// The display label of the option.
    pub label: String,
}

/// One settings key declared by a plugin manifest.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SettingDeclaration {
    /// Plugin-local settings key; dotted paths address nested struct
    /// fields (e.g. `interface.theme_id`).
    pub key: String,
    /// The control kind rendered for this key.
    pub kind: SettingKind,
    /// Numeric bounds for [`SettingKind::Number`] declarations.
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    #[serde(default)]
    pub step: Option<f64>,
    /// Optional unit suffix for [`SettingKind::Number`] declarations.
    #[serde(default)]
    pub unit: Option<String>,
    /// The selectable options of [`SettingKind::Enum`] declarations.
    #[serde(default)]
    pub options: Vec<SettingOption>,
    /// The default value; the host falls back to it when the stored value
    /// is missing or fails validation.
    pub default: serde_json::Value,
    /// Display title shown next to the control.
    pub title: String,
    /// Optional display description shown under the title.
    #[serde(default)]
    pub description: Option<String>,
}

impl SettingDeclaration {
    /// Whether `value` satisfies this declaration's kind and bounds.
    pub fn accepts(&self, value: &Value) -> bool {
        match &self.kind {
            SettingKind::Bool => value.is_boolean(),
            SettingKind::Number => value.is_number(),
            SettingKind::String
            | SettingKind::Font
            | SettingKind::Theme
            | SettingKind::Language => value.is_string(),
            SettingKind::Enum => value
                .as_str()
                .is_some_and(|value| self.options.iter().any(|option| option.value == value)),
        }
    }
}

/// Verifies that a plugin's typed settings struct and its manifest
/// declarations cover each other exactly, returning a list of mismatches:
///
/// - every declared key must exist in the struct (enforced through
///   `serde(deny_unknown_fields)` on the struct);
/// - every leaf field of the struct must have a declaration, except paths
///   listed in `unexposed` (config-only channels that the settings UI never
///   renders, such as keybinding overrides).
pub fn verify_setting_declarations<T>(
    declarations: &[SettingDeclaration],
    unexposed: &[&str],
) -> Vec<String>
where
    T: Serialize + for<'de> Deserialize<'de> + Default,
{
    let mut problems = Vec::new();

    // Declared defaults must deserialize into the struct. The dotted keys
    // are first expanded into nested objects matching the struct shape.
    let mut object = serde_json::Map::new();
    for declaration in declarations {
        let mut segments = declaration.key.split('.').collect::<Vec<_>>();
        let leaf = segments.pop().expect("settings key must not be empty");
        let mut current = &mut object;
        for segment in segments {
            current = current
                .entry(segment.to_string())
                .or_insert_with(|| Value::Object(Default::default()))
                .as_object_mut()
                .expect("just ensured object");
        }
        current.insert(leaf.to_string(), declaration.default.clone());
    }
    if serde_json::from_value::<T>(Value::Object(object)).is_err() {
        problems.push(format!(
            "declared setting defaults do not deserialize into {}",
            std::any::type_name::<T>()
        ));
    }

    // Every leaf field of the struct must be declared.
    if let Ok(default) = serde_json::to_value(T::default()) {
        collect_undeclared_leaves(
            &default,
            "",
            unexposed,
            declarations,
            std::any::type_name::<T>(),
            &mut problems,
        );
    }

    problems
}

fn collect_undeclared_leaves(
    value: &Value,
    prefix: &str,
    unexposed: &[&str],
    declarations: &[SettingDeclaration],
    struct_name: &str,
    problems: &mut Vec<String>,
) {
    let Value::Object(fields) = value else {
        return;
    };
    for (field, child) in fields {
        let path = if prefix.is_empty() {
            field.clone()
        } else {
            format!("{prefix}.{field}")
        };
        if unexposed
            .iter()
            .any(|hidden| path == *hidden || path.starts_with(&format!("{hidden}.")))
        {
            continue;
        }
        let is_nested_object = matches!(child, Value::Object(map) if !map.is_empty());
        if is_nested_object {
            collect_undeclared_leaves(child, &path, unexposed, declarations, struct_name, problems);
        } else if !declarations
            .iter()
            .any(|declaration| declaration.key == path)
        {
            problems.push(format!(
                "field '{path}' of {struct_name} has no setting declaration"
            ));
        }
    }
}
