use schemars::{gen::SchemaSettings, schema::SchemaObject};
use serde::Serialize;

pub use schemars as type_def;
pub use schemars::JsonSchema;

#[derive(Debug, Clone, Serialize)]
pub struct Doc {
    pub openrpc: String,
    pub info: Info,
    pub methods: Vec<Method>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Info {
    pub version: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Method {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub params: Vec<Param>,
    pub result: Param,
    pub param_structure: ParamStructure,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ParamStructure {
    ByName,
    ByPosition,
}

#[derive(Debug, Clone, Serialize)]
pub struct Param {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub schema: SchemaObject,
    pub required: bool,
}

pub fn object_schema_to_params<T: JsonSchema>() -> anyhow::Result<Vec<Param>> {
    let schema = generate_schema::<T>();
    let properties = match schema.object.as_ref() {
        Some(obj) => &obj.properties,
        None => return Err(anyhow::anyhow!("Invalid parameter definition")),
    };
    let mut params = vec![];
    for (key, schema) in properties {
        params.push(Param {
            name: key.to_string(),
            schema: schema.clone().into_object(),
            description: None,
            required: true,
        });
    }
    Ok(params)
}

pub fn generate_schema<T: JsonSchema>() -> SchemaObject {
    let settings = SchemaSettings::draft07().with(|s| {
        s.inline_subschemas = true;
    });
    let gen = settings.into_generator();
    let schema = gen.into_root_schema_for::<T>();
    schema.schema
}
