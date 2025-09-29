use std::path::PathBuf;

use platynui_spy_core::{
    capture_tree, AppConfig, AttributeConfig, AttributeSet, BackendError, BackendKind,
    FilterConfig, XPath,
};
#[cfg(target_os = "windows")]
use platynui_spy_core::{Win32Config, Win32Root};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyType};
use serde_json::Value as JsonValue;

#[pyclass]
#[derive(Clone)]
struct SpyConfig {
    backend: BackendKind,
    input: Option<PathBuf>,
    max_depth: Option<usize>,
    include_ancestors: bool,
    filter_name: Option<String>,
    filter_role: Option<String>,
    filter_attrs: Vec<(String, String)>,
    attribute_set: AttributeSet,
    attribute_keys: Vec<String>,
    xpath: Option<String>,
    #[cfg(target_os = "windows")]
    win32_root: Win32Root,
    #[cfg(target_os = "windows")]
    win32_process_id: Option<u32>,
    #[cfg(target_os = "windows")]
    win32_window_title: Option<String>,
    #[cfg(target_os = "windows")]
    win32_top_level_only: bool,
}

impl Default for SpyConfig {
    fn default() -> Self {
        Self {
            backend: BackendKind::File,
            input: None,
            max_depth: None,
            include_ancestors: true,
            filter_name: None,
            filter_role: None,
            filter_attrs: Vec::new(),
            attribute_set: AttributeSet::Essential,
            attribute_keys: Vec::new(),
            xpath: None,
            #[cfg(target_os = "windows")]
            win32_root: Win32Root::Desktop,
            #[cfg(target_os = "windows")]
            win32_process_id: None,
            #[cfg(target_os = "windows")]
            win32_window_title: None,
            #[cfg(target_os = "windows")]
            win32_top_level_only: false,
        }
    }
}

#[pymethods]
impl SpyConfig {
    #[new]
    fn new() -> Self {
        Self::default()
    }

    #[classmethod]
    fn backend(_cls: &Bound<PyType>, backend: &str) -> PyResult<Self> {
        let backend = parse_backend(backend)?;
        Ok(Self {
            backend,
            ..Self::default()
        })
    }

    fn with_backend(&self, backend: &str) -> PyResult<Self> {
        let mut cfg = self.clone();
        cfg.backend = parse_backend(backend)?;
        Ok(cfg)
    }

    fn with_input(&self, path: &str) -> PyResult<Self> {
        if path.trim().is_empty() {
            return Err(PyValueError::new_err("input path cannot be empty"));
        }
        let mut cfg = self.clone();
        cfg.input = Some(PathBuf::from(path));
        Ok(cfg)
    }

    fn with_max_depth(&self, depth: Option<usize>) -> Self {
        let mut cfg = self.clone();
        cfg.max_depth = depth;
        cfg
    }

    fn include_ancestors(&self, include: bool) -> Self {
        let mut cfg = self.clone();
        cfg.include_ancestors = include;
        cfg
    }

    fn with_name_filter(&self, name: &str) -> Self {
        let mut cfg = self.clone();
        let trimmed = name.trim();
        cfg.filter_name = if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_lowercase())
        };
        cfg
    }

    fn with_role_filter(&self, role: &str) -> Self {
        let mut cfg = self.clone();
        let trimmed = role.trim();
        cfg.filter_role = if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_lowercase())
        };
        cfg
    }

    fn add_attribute_filter(&self, key: &str, value: &str) -> PyResult<Self> {
        let key = key.trim();
        let value = value.trim();
        if key.is_empty() || value.is_empty() {
            return Err(PyValueError::new_err(
                "attribute filters require non-empty key and value",
            ));
        }
        let mut cfg = self.clone();
        cfg.filter_attrs.push((key.to_string(), value.to_string()));
        Ok(cfg)
    }

    fn clear_attribute_filters(&self) -> Self {
        let mut cfg = self.clone();
        cfg.filter_attrs.clear();
        cfg
    }

    fn with_attribute_set(&self, set: &str) -> PyResult<Self> {
        let mut cfg = self.clone();
        cfg.attribute_set = parse_attribute_set(set)?;
        Ok(cfg)
    }

    fn add_attribute_key(&self, key: &str) -> Self {
        let mut cfg = self.clone();
        let trimmed = key.trim();
        if !trimmed.is_empty() {
            cfg.attribute_keys.push(trimmed.to_string());
        }
        cfg
    }

    fn clear_attribute_keys(&self) -> Self {
        let mut cfg = self.clone();
        cfg.attribute_keys.clear();
        cfg
    }

    fn with_xpath(&self, xpath: &str) -> Self {
        let mut cfg = self.clone();
        let trimmed = xpath.trim();
        if trimmed.is_empty() {
            cfg.xpath = None;
        } else {
            cfg.xpath = Some(trimmed.to_string());
        }
        cfg
    }

    #[cfg(target_os = "windows")]
    fn with_process_id(&self, process_id: Option<u32>) -> Self {
        let mut cfg = self.clone();
        cfg.win32_process_id = process_id;
        cfg
    }

    #[cfg(target_os = "windows")]
    fn with_window_title(&self, title: Option<&str>) -> Self {
        let mut cfg = self.clone();
        cfg.win32_window_title = title.map(|value| value.to_string());
        cfg
    }

    #[cfg(target_os = "windows")]
    fn with_top_level_only(&self, top_level_only: bool) -> Self {
        let mut cfg = self.clone();
        cfg.win32_top_level_only = top_level_only;
        cfg
    }

    #[cfg(target_os = "windows")]
    fn with_root(&self, root: &str) -> PyResult<Self> {
        let mut cfg = self.clone();
        cfg.win32_root = parse_win32_root(root)?;
        Ok(cfg)
    }
}

impl SpyConfig {
    fn to_core_config(&self) -> PyResult<AppConfig> {
        if matches!(self.backend, BackendKind::File) && self.input.is_none() {
            return Err(PyValueError::new_err(
                "file backend requires an input path via with_input()",
            ));
        }

        if is_win32_backend(&self.backend) {
            #[cfg(not(target_os = "windows"))]
            {
                return Err(PyValueError::new_err(
                    "the win32 backend is only available on Windows",
                ));
            }
        }

        let filter = FilterConfig::new(
            self.max_depth,
            self.include_ancestors,
            self.filter_name.clone(),
            self.filter_role.clone(),
            self.filter_attrs.clone(),
        );

        let attributes =
            AttributeConfig::new(self.attribute_set.clone(), self.attribute_keys.clone());

        let xpath = match &self.xpath {
            Some(expr) => {
                Some(XPath::parse(expr).map_err(|err| PyValueError::new_err(err.to_string()))?)
            }
            None => None,
        };

        #[cfg(target_os = "windows")]
        let win32 = Win32Config {
            root: self.win32_root,
            process_id: self.win32_process_id,
            window_title: self
                .win32_window_title
                .as_ref()
                .map(|value| value.trim().to_lowercase())
                .filter(|value| !value.is_empty()),
            top_level_only: self.win32_top_level_only,
        };

        Ok(AppConfig {
            backend: self.backend.clone(),
            input: self.input.clone(),
            filter,
            attributes,
            xpath,
            #[cfg(target_os = "windows")]
            win32,
        })
    }
}

#[pyfunction]
fn capture(py: Python<'_>, config: &SpyConfig) -> PyResult<Option<PyObject>> {
    let core = config.to_core_config()?;
    let tree = capture_tree(&core).map_err(map_backend_error)?;
    match tree {
        Some(node) => {
            let json = serde_json::to_value(&node)
                .map_err(|err| PyRuntimeError::new_err(err.to_string()))?;
            let value = json_to_py(py, &json)?;
            Ok(Some(value))
        }
        None => Ok(None),
    }
}

fn map_backend_error(err: BackendError) -> PyErr {
    match err {
        BackendError::MissingInput => PyValueError::new_err("missing input for file backend"),
        BackendError::ReadFailure { path, source } => {
            PyRuntimeError::new_err(format!("failed to read input {:?}: {}", path, source))
        }
        BackendError::ParseFailure { path, source } => {
            PyRuntimeError::new_err(format!("failed to parse UI tree {:?}: {}", path, source))
        }
        #[cfg(target_os = "windows")]
        BackendError::WindowsAutomation { source } => {
            PyRuntimeError::new_err(format!("UI Automation capture failed: {source}"))
        }
        #[cfg(target_os = "windows")]
        BackendError::WindowsTargetNotFound { selectors } => PyRuntimeError::new_err(format!(
            "no Windows UI element matched the provided selectors: {selectors}"
        )),
    }
}

fn json_to_py(py: Python<'_>, value: &JsonValue) -> PyResult<PyObject> {
    match value {
        JsonValue::Null => Ok(py.None()),
        JsonValue::Bool(v) => Ok(v.into_py(py)),
        JsonValue::Number(num) => {
            if let Some(i) = num.as_i64() {
                Ok(i.into_py(py))
            } else if let Some(u) = num.as_u64() {
                Ok(u.into_py(py))
            } else if let Some(f) = num.as_f64() {
                Ok(f.into_py(py))
            } else {
                Err(PyRuntimeError::new_err("unsupported JSON number"))
            }
        }
        JsonValue::String(s) => Ok(s.into_py(py)),
        JsonValue::Array(items) => {
            let list = PyList::empty_bound(py);
            for item in items {
                list.append(json_to_py(py, item)?)?;
            }
            Ok(list.into_py(py))
        }
        JsonValue::Object(map) => {
            let dict = PyDict::new_bound(py);
            for (key, value) in map {
                dict.set_item(key, json_to_py(py, value)?)?;
            }
            Ok(dict.into_py(py))
        }
    }
}

#[cfg(target_os = "windows")]
fn is_win32_backend(backend: &BackendKind) -> bool {
    matches!(backend, BackendKind::Win32)
}

#[cfg(not(target_os = "windows"))]
fn is_win32_backend(_backend: &BackendKind) -> bool {
    false
}

fn parse_backend(input: &str) -> PyResult<BackendKind> {
    match input.to_ascii_lowercase().as_str() {
        "file" => Ok(BackendKind::File),
        #[cfg(target_os = "windows")]
        "win32" => Ok(BackendKind::Win32),
        _ => Err(PyValueError::new_err(
            "supported backends are 'file' and 'win32'",
        )),
    }
}

fn parse_attribute_set(input: &str) -> PyResult<AttributeSet> {
    match input.to_ascii_lowercase().as_str() {
        "none" => Ok(AttributeSet::None),
        "essential" => Ok(AttributeSet::Essential),
        "full" => Ok(AttributeSet::Full),
        _ => Err(PyValueError::new_err(
            "attribute set must be 'none', 'essential', or 'full'",
        )),
    }
}

#[cfg(target_os = "windows")]
fn parse_win32_root(input: &str) -> PyResult<Win32Root> {
    match input.to_ascii_lowercase().as_str() {
        "desktop" => Ok(Win32Root::Desktop),
        "focused" => Ok(Win32Root::Focused),
        _ => Err(PyValueError::new_err(
            "win32 root must be 'desktop' or 'focused'",
        )),
    }
}

#[pymodule]
fn platynui_spy(py: Python<'_>, m: &Bound<PyModule>) -> PyResult<()> {
    m.add_class::<SpyConfig>()?;
    m.add_function(wrap_pyfunction!(capture, m)?)?;

    // Provide the essential attribute defaults as a Python tuple for convenience.
    let essentials: Vec<&str> = platynui_spy_core::ESSENTIAL_ATTRIBUTES.to_vec();
    m.add("ESSENTIAL_ATTRIBUTES", essentials.into_py(py))?;

    Ok(())
}

#[cfg(all(test, feature = "python-tests"))]
mod tests {
    use super::*;
    use serde_json::{Map as JsonMap, Number as JsonNumber, Value as JsonValue};

    fn sample_tree_path() -> PathBuf {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest
            .join("../platynui-spy-cli/tests/data/sample_tree.json")
            .canonicalize()
            .expect("sample tree")
    }

    fn py_to_json(value: &Bound<PyAny>) -> PyResult<JsonValue> {
        if value.is_none() {
            return Ok(JsonValue::Null);
        }

        if let Ok(dict) = value.downcast::<PyDict>() {
            let mut map = JsonMap::new();
            for (key, val) in dict.iter() {
                let key_str: String = key.extract()?;
                map.insert(key_str, py_to_json(&val)?);
            }
            return Ok(JsonValue::Object(map));
        }

        if let Ok(list) = value.downcast::<PyList>() {
            let mut items = Vec::new();
            for item in list.iter() {
                items.push(py_to_json(&item)?);
            }
            return Ok(JsonValue::Array(items));
        }

        if let Ok(s) = value.extract::<String>() {
            return Ok(JsonValue::String(s));
        }

        if let Ok(b) = value.extract::<bool>() {
            return Ok(JsonValue::Bool(b));
        }

        if let Ok(i) = value.extract::<i64>() {
            return Ok(JsonValue::Number(i.into()));
        }

        if let Ok(u) = value.extract::<u64>() {
            return Ok(JsonValue::Number(u.into()));
        }

        if let Ok(f) = value.extract::<f64>() {
            let number = JsonNumber::from_f64(f)
                .ok_or_else(|| PyRuntimeError::new_err("invalid floating point value"))?;
            return Ok(JsonValue::Number(number));
        }

        Err(PyRuntimeError::new_err(
            "unsupported Python value in UI tree",
        ))
    }

    #[test]
    fn builds_core_config_from_defaults() {
        let cfg = SpyConfig::default();
        let result = cfg.to_core_config();
        assert!(result.is_err(), "file backend requires input");
    }

    #[test]
    fn capture_from_json_file_returns_tree() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let path = sample_tree_path();
            let cfg = SpyConfig::default()
                .with_input(path.to_str().unwrap())
                .expect("input")
                .with_attribute_set("full")
                .expect("attribute set");
            let result = capture(py, &cfg).expect("capture result");
            let value = result.expect("tree available");
            let json: JsonValue = py_to_json(value.bind(py)).expect("extract json");
            assert_eq!(json["name"], JsonValue::String("Calculator".into()));
        });
    }
}
