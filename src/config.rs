use serde::Deserialize;
use std::error::Error;
use std::fs;
use std::path::Path;

/// Root configuration structure
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub app: AppConfig,
    pub columns: Vec<Column>,
}

/// Application-level settings (title, subtitle, etc.)
#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub title: String,
    pub subtitle: String,
}

/// A column in the UI (e.g., Projects, Servers, Tools)
#[derive(Debug, Deserialize, Clone)]
pub struct Column {
    pub id: String,
    pub title: String,
    pub actions: Vec<Action>,
}

/// An action within a column
#[derive(Debug, Deserialize, Clone)]
pub struct Action {
    pub label: String,
    pub template: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub parameters: Vec<Parameter>,
}

/// Parameter type: text input or dropdown select
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ParameterType {
    Text,
    Select,
}

impl Default for ParameterType {
    fn default() -> Self {
        ParameterType::Text
    }
}

/// A parameter for an action (placeholder to be replaced in template)
#[derive(Debug, Deserialize, Clone)]
pub struct Parameter {
    pub name: String,
    pub placeholder: String,
    #[serde(default)]
    pub param_type: ParameterType,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub options: Vec<ParameterOption>,
    #[serde(default)]
    pub default: Option<String>,
}

/// Option for select-type parameters
#[derive(Debug, Deserialize, Clone)]
pub struct ParameterOption {
    pub value: String,
    pub label: String,
}

impl Config {
    /// Load configuration from a TOML file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(format!(
                "Configuration file not found: {}\n\
                 Please create a config.toml file in the same directory as the executable.",
                path.display()
            )
            .into());
        }

        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file '{}': {}", path.display(), e))?;

        let config: Config = toml::from_str(&content)
            .map_err(|e| format!("Failed to parse config file '{}': {}", path.display(), e))?;

        // Validate the config
        config.validate()?;

        Ok(config)
    }

    /// Validate the configuration
    fn validate(&self) -> Result<(), Box<dyn Error>> {
        if self.columns.is_empty() {
            return Err("Configuration must have at least one column".into());
        }

        for column in &self.columns {
            if column.id.is_empty() {
                return Err("Column id cannot be empty".into());
            }
            if column.title.is_empty() {
                return Err(format!("Column '{}' must have a title", column.id).into());
            }
            if column.actions.is_empty() {
                return Err(format!("Column '{}' must have at least one action", column.id).into());
            }

            for action in &column.actions {
                if action.label.is_empty() {
                    return Err(
                        format!("Action in column '{}' must have a label", column.id).into(),
                    );
                }
                if action.template.is_empty() {
                    return Err(format!(
                        "Action '{}' in column '{}' must have a template",
                        action.label, column.id
                    )
                    .into());
                }

                // Validate parameters
                for param in &action.parameters {
                    if param.name.is_empty() {
                        return Err(format!(
                            "Parameter in action '{}' must have a name",
                            action.label
                        )
                        .into());
                    }
                    if param.placeholder.is_empty() {
                        return Err(format!(
                            "Parameter '{}' in action '{}' must have a placeholder",
                            param.name, action.label
                        )
                        .into());
                    }
                    // Select type must have options
                    if param.param_type == ParameterType::Select && param.options.is_empty() {
                        return Err(format!(
                            "Parameter '{}' in action '{}' is type 'select' but has no options",
                            param.name, action.label
                        )
                        .into());
                    }
                }
            }
        }

        Ok(())
    }
}
