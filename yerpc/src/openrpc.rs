use schemars::{gen::SchemaSettings, schema::SchemaObject, Map};
use serde::Serialize;

pub use schemars as type_def;
pub use schemars::JsonSchema;

/// [OpenRPC object](https://spec.open-rpc.org/#openrpc-object),
/// the root of OpenRPC document.
#[derive(Debug, Clone, Serialize)]
pub struct Doc {
    pub openrpc: String,
    pub info: Info,
    pub methods: Vec<Method>,
    pub components: Components,
}

/// [Info Object](https://spec.open-rpc.org/#info-object)
#[derive(Debug, Clone, Serialize)]
pub struct Info {
    /// OpenRPC document version.
    pub version: String,

    /// Application title.
    pub title: String,
}

/// [Method Object](https://spec.open-rpc.org/#method-object)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Method {
    /// Method name.
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub params: Vec<Param>,
    pub result: Param,

    /// Whether request params are an array or an object.
    pub param_structure: ParamStructure,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ParamStructure {
    /// Request params are an object.
    ByName,

    /// Request params are an array.
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

#[derive(Debug, Clone, Serialize)]
pub struct Components {
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub schemas: Map<String, SchemaObject>,
}

pub fn object_schema_to_params<T: JsonSchema>(
) -> anyhow::Result<(Vec<Param>, Map<String, SchemaObject>)> {
    let (schema, definitions) = generate_schema::<T>();
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
    Ok((params, definitions))
}

/// Generates a single schema.
///
/// Returns schema object and referenced definitions
/// to be put into `schemas` field
/// of the [Components Object](https://spec.open-rpc.org/#components-object).
pub fn generate_schema<T: JsonSchema>() -> (SchemaObject, Map<String, SchemaObject>) {
    let settings = SchemaSettings::draft07().with(|s| {
        s.inline_subschemas = false;
        s.definitions_path = "#/components/schemas/".to_string();
    });
    let gen = settings.into_generator();
    let schema = gen.into_root_schema_for::<T>();
    let definitions: Map<String, SchemaObject> = schema
        .definitions
        .into_iter()
        .map(|(k, v)| (k, v.into_object()))
        .collect();
    (schema.schema, definitions)
}
