//! OpenAPI 3.0 spec generation from contract ABI.
//!
//! Produces OpenAPI YAML/JSON from ContractABI for documentation and Swagger UI.

use serde::Serialize;
use std::collections::BTreeMap;

use super::types::*;

/// OpenAPI 3.0 root document
#[derive(Debug, Clone, Serialize)]
pub struct OpenApiDoc {
    pub openapi: String,
    pub info: OpenApiInfo,
    pub paths: BTreeMap<String, PathItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<OpenApiComponents>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenApiInfo {
    pub title: String,
    pub description: Option<String>,
    pub version: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PathItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub get: Option<Operation>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Operation {
    pub operation_id: String,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_body: Option<RequestBody>,
    pub responses: BTreeMap<String, Response>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RequestBody {
    pub required: bool,
    pub content: BTreeMap<String, MediaType>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MediaType {
    pub schema: SchemaRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub examples: Option<BTreeMap<String, Example>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Example {
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct Response {
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<BTreeMap<String, MediaType>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum SchemaRef {
    Inline(Box<Schema>),
    Ref {
        #[serde(rename = "$ref")]
        r#ref: String,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct Schema {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<BTreeMap<String, SchemaRef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<SchemaRef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_properties: Option<Box<SchemaRef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nullable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<serde_json::Value>,
    #[serde(rename = "$ref", skip_serializing_if = "Option::is_none")]
    pub ref_: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenApiComponents {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schemas: Option<BTreeMap<String, Schema>>,
}

/// Generate OpenAPI 3.0 document from ContractABI
pub fn generate_openapi(abi: &ContractABI, base_path: Option<&str>) -> OpenApiDoc {
    let base = base_path.unwrap_or("/invoke");
    let mut paths = BTreeMap::new();
    let mut schema_gen = SchemaGenerator::new();

    for func in abi.public_functions() {
        let path = format!("{}/{}", base.trim_end_matches('/'), func.name);
        let op = operation_from_function(func, abi, &mut schema_gen);
        paths.insert(path, PathItem { post: Some(op), get: None });
    }

    let components = schema_gen.into_components();
    let info = OpenApiInfo {
        title: abi.name.clone(),
        description: abi.version.as_ref().map(|v| format!("Contract ABI (version {})", v)),
        version: abi.version.clone().unwrap_or_else(|| "0.0.0".to_string()),
    };

    OpenApiDoc {
        openapi: "3.0.0".to_string(),
        info,
        paths,
        components: if components.schemas.as_ref().map_or(true, |s| s.is_empty()) {
            None
        } else {
            Some(components)
        },
    }
}

fn operation_from_function(
    func: &ContractFunction,
    abi: &ContractABI,
    schema_gen: &mut SchemaGenerator,
) -> Operation {
    let operation_id = func.name.clone();
    let summary = func.doc.as_deref().unwrap_or(&func.name).to_string();
    let description = func.doc.clone();

    let (request_body, _request_example) = if func.params.is_empty() {
        (None, None)
    } else {
        let (schema_ref, example) = schema_gen.params_schema_and_example(&func.params);
        let content = BTreeMap::from([(
            "application/json".to_string(),
            MediaType {
                schema: schema_ref,
                example: example.clone(),
                examples: None,
            },
        )]);
        let example = example;
        (
            Some(RequestBody {
                required: true,
                content,
            }),
            example,
        )
    };

    let mut responses = BTreeMap::new();
    let (response_schema, response_example) = schema_gen.type_to_schema_and_example(&func.return_type);
    responses.insert(
        "200".to_string(),
        Response {
            description: format!("Success. Returns: {}", func.return_type.display_name()),
            content: Some(BTreeMap::from([(
                "application/json".to_string(),
                MediaType {
                    schema: response_schema,
                    example: response_example,
                    examples: None,
                },
            )])),
        },
    );

    // Contract errors as 4xx/5xx
    if !abi.errors.is_empty() {
        let err_desc: String = abi
            .errors
            .iter()
            .map(|e| format!("{} (code {}): {}", e.name, e.code, e.doc.as_deref().unwrap_or("")))
            .collect::<Vec<_>>()
            .join("; ");
        responses.insert(
            "400".to_string(),
            Response {
                description: format!("Contract error. {}", err_desc),
                content: None,
            },
        );
    }

    Operation {
        operation_id,
        summary: summary.lines().next().unwrap_or("").to_string(),
        description,
        request_body,
        responses,
        tags: Some(vec!["Contract".to_string()]),
    }
}

struct SchemaGenerator {
    schemas: BTreeMap<String, Schema>,
    next_id: usize,
}

impl SchemaGenerator {
    fn new() -> Self {
        Self {
            schemas: BTreeMap::new(),
            next_id: 0,
        }
    }

    fn params_schema_and_example(&mut self, params: &[FunctionParam]) -> (SchemaRef, Option<serde_json::Value>) {
        let mut properties = BTreeMap::new();
        let mut required = Vec::new();
        let mut example = serde_json::Map::new();
        for p in params {
            let (schema, ex) = self.type_to_schema_and_example(&p.param_type);
            properties.insert(p.name.clone(), schema);
            required.push(p.name.clone());
            if let Some(ex) = ex {
                example.insert(p.name.clone(), ex);
            }
        }
        let schema = Schema {
            r#type: Some("object".to_string()),
            format: None,
            description: None,
            properties: Some(properties),
            required: Some(required),
            items: None,
            additional_properties: None,
            nullable: None,
            example: if example.is_empty() {
                None
            } else {
                Some(serde_json::Value::Object(example))
            },
            ref_: None,
        };
        let ex = schema.example.clone();
        (SchemaRef::Inline(Box::new(schema)), ex)
    }

    fn type_to_schema_and_example(&mut self, t: &SorobanType) -> (SchemaRef, Option<serde_json::Value>) {
        match t {
            SorobanType::Bool => (
                SchemaRef::Inline(Box::new(Schema {
                    r#type: Some("boolean".to_string()),
                    format: None,
                    description: None,
                    properties: None,
                    required: None,
                    items: None,
                    additional_properties: None,
                    nullable: None,
                    example: Some(serde_json::Value::Bool(false)),
                    ref_: None,
                })),
                Some(serde_json::Value::Bool(false)),
            ),
            SorobanType::I32 | SorobanType::I64 | SorobanType::I128 | SorobanType::I256 => {
                let format = match t {
                    SorobanType::I32 => "int32",
                    SorobanType::I64 => "int64",
                    _ => "string",
                };
                (
                    SchemaRef::Inline(Box::new(Schema {
                        r#type: Some("integer".to_string()),
                        format: Some(format.to_string()),
                        description: Some(t.display_name()),
                        properties: None,
                        required: None,
                        items: None,
                        additional_properties: None,
                        nullable: None,
                        example: Some(serde_json::Value::Number(0.into())),
                        ref_: None,
                    })),
                    Some(serde_json::Value::Number(0.into())),
                )
            }
            SorobanType::U32 | SorobanType::U64 | SorobanType::U128 | SorobanType::U256 => (
                SchemaRef::Inline(Box::new(Schema {
                    r#type: Some("integer".to_string()),
                    format: Some("int64".to_string()),
                    description: Some(t.display_name()),
                    properties: None,
                    required: None,
                    items: None,
                    additional_properties: None,
                    nullable: None,
                    example: Some(serde_json::Value::Number(0.into())),
                    ref_: None,
                })),
                Some(serde_json::Value::Number(0.into())),
            ),
            SorobanType::String | SorobanType::Symbol => (
                SchemaRef::Inline(Box::new(Schema {
                    r#type: Some("string".to_string()),
                    format: None,
                    description: Some(t.display_name()),
                    properties: None,
                    required: None,
                    items: None,
                    additional_properties: None,
                    nullable: None,
                    example: Some(serde_json::Value::String(String::new())),
                    ref_: None,
                })),
                Some(serde_json::Value::String(String::new())),
            ),
            SorobanType::Address => (
                SchemaRef::Inline(Box::new(Schema {
                    r#type: Some("string".to_string()),
                    format: Some("stellar-address".to_string()),
                    description: Some("Stellar address (account or contract)".to_string()),
                    properties: None,
                    required: None,
                    items: None,
                    additional_properties: None,
                    nullable: None,
                    example: Some(serde_json::Value::String("C...".to_string())),
                    ref_: None,
                })),
                Some(serde_json::Value::String("C...".to_string())),
            ),
            SorobanType::Bytes | SorobanType::BytesN { .. } => (
                SchemaRef::Inline(Box::new(Schema {
                    r#type: Some("string".to_string()),
                    format: Some("byte".to_string()),
                    description: Some(t.display_name()),
                    properties: None,
                    required: None,
                    items: None,
                    additional_properties: None,
                    nullable: None,
                    example: Some(serde_json::Value::String("base64...".to_string())),
                    ref_: None,
                })),
                Some(serde_json::Value::String("base64...".to_string())),
            ),
            SorobanType::Void => (
                SchemaRef::Inline(Box::new(Schema {
                    r#type: Some("object".to_string()),
                    format: None,
                    description: Some("void".to_string()),
                    properties: None,
                    required: None,
                    items: None,
                    additional_properties: None,
                    nullable: Some(true),
                    example: None,
                    ref_: None,
                })),
                None,
            ),
            SorobanType::Timepoint | SorobanType::Duration => (
                SchemaRef::Inline(Box::new(Schema {
                    r#type: Some("integer".to_string()),
                    format: None,
                    description: Some(t.display_name()),
                    properties: None,
                    required: None,
                    items: None,
                    additional_properties: None,
                    nullable: None,
                    example: Some(serde_json::Value::Number(0.into())),
                    ref_: None,
                })),
                Some(serde_json::Value::Number(0.into())),
            ),
            SorobanType::Option { value_type } => {
                let (inner, ex) = self.type_to_schema_and_example(value_type);
                // Represent Option<T> as nullable T
                let nullable_schema = match inner {
                    SchemaRef::Inline(mut s) => {
                        s.nullable = Some(true);
                        SchemaRef::Inline(s)
                    }
                    SchemaRef::Ref { r#ref } => {
                        // Cannot add nullable to $ref in OpenAPI 3.0 without allOf; use inline wrapper
                        SchemaRef::Inline(Box::new(Schema {
                            r#type: None,
                            format: None,
                            description: Some(t.display_name()),
                            properties: None,
                            required: None,
                            items: None,
                            additional_properties: None,
                            nullable: Some(true),
                            example: ex.clone(),
                            ref_: Some(r#ref),
                        }))
                    }
                };
                (nullable_schema, ex.or(Some(serde_json::Value::Null)))
            }
            SorobanType::Vec { element_type } => {
                let (item_schema, item_ex) = self.type_to_schema_and_example(element_type);
                let arr_ex = item_ex.map(|e| serde_json::Value::Array(vec![e]));
                (
                    SchemaRef::Inline(Box::new(Schema {
                        r#type: Some("array".to_string()),
                        format: None,
                        description: Some(t.display_name()),
                        properties: None,
                        required: None,
                        items: Some(Box::new(item_schema)),
                        additional_properties: None,
                        nullable: None,
                        example: arr_ex.clone(),
                        ref_: None,
                    })),
                    arr_ex,
                )
            }
            SorobanType::Map { key_type: _, value_type } => {
                let (val_schema, _) = self.type_to_schema_and_example(value_type);
                (
                    SchemaRef::Inline(Box::new(Schema {
                        r#type: Some("object".to_string()),
                        format: None,
                        description: Some(t.display_name()),
                        properties: None,
                        required: None,
                        items: None,
                        additional_properties: Some(Box::new(val_schema)),
                        nullable: None,
                        example: Some(serde_json::Value::Object(serde_json::Map::new())),
                        ref_: None,
                    })),
                    Some(serde_json::Value::Object(serde_json::Map::new())),
                )
            }
            SorobanType::Struct { name, fields } => {
                let schema_name = sanitize_schema_name(name);
                if !self.schemas.contains_key(&schema_name) {
                    let mut properties = BTreeMap::new();
                    let mut required = Vec::new();
                    let mut ex_map = serde_json::Map::new();
                    for f in fields {
                        let (s, ex) = self.type_to_schema_and_example(&f.field_type);
                        properties.insert(f.name.clone(), s);
                        required.push(f.name.clone());
                        if let Some(ex) = ex {
                            ex_map.insert(f.name.clone(), ex);
                        }
                    }
                    self.schemas.insert(
                        schema_name.clone(),
                        Schema {
                            r#type: Some("object".to_string()),
                            format: None,
                            description: Some(name.clone()),
                            properties: Some(properties),
                            required: Some(required),
                            items: None,
                            additional_properties: None,
                            nullable: None,
                            example: if ex_map.is_empty() {
                                None
                            } else {
                                Some(serde_json::Value::Object(ex_map))
                            },
                            ref_: None,
                        },
                    );
                }
                let ref_path = format!("#/components/schemas/{}", schema_name);
                let ex = self.schemas.get(&schema_name).and_then(|s| s.example.clone());
                (
                    SchemaRef::Ref {
                        r#ref: ref_path,
                    },
                    ex,
                )
            }
            SorobanType::Enum { name, variants } => {
                let schema_name = sanitize_schema_name(name);
                if !self.schemas.contains_key(&schema_name) {
                    let enum_vals: Vec<serde_json::Value> =
                        variants.iter().map(|v| serde_json::Value::String(v.name.clone())).collect();
                    let ex = enum_vals.first().cloned();
                    self.schemas.insert(
                        schema_name.clone(),
                        Schema {
                            r#type: Some("string".to_string()),
                            format: None,
                            description: Some(
                                variants
                                    .iter()
                                    .map(|v| format!("{}", v.name))
                                    .collect::<Vec<_>>()
                                    .join(", "),
                            ),
                            properties: None,
                            required: None,
                            items: None,
                            additional_properties: None,
                            nullable: None,
                            example: ex,
                            ref_: None,
                        },
                    );
                }
                let ref_path = format!("#/components/schemas/{}", schema_name);
                let ex = self.schemas.get(&schema_name).and_then(|s| s.example.clone());
                (
                    SchemaRef::Ref {
                        r#ref: ref_path,
                    },
                    ex,
                )
            }
            SorobanType::Tuple { elements } => {
                let (item_schemas, item_examples): (Vec<_>, Vec<_>) = elements
                    .iter()
                    .map(|e| self.type_to_schema_and_example(e))
                    .unzip();
                let arr_ex = Some(serde_json::Value::Array(
                    item_examples.into_iter().flatten().collect(),
                ));
                let schema = Schema {
                    r#type: Some("array".to_string()),
                    format: None,
                    description: Some(t.display_name()),
                    properties: None,
                    required: None,
                    items: if item_schemas.is_empty() {
                        None
                    } else {
                        Some(Box::new(SchemaRef::Inline(Box::new(Schema {
                            r#type: Some("object".to_string()),
                            format: None,
                            description: None,
                            properties: None,
                            required: None,
                            items: None,
                            additional_properties: None,
                            nullable: None,
                            example: None,
                            ref_: None,
                        }))))
                    },
                    additional_properties: None,
                    nullable: None,
                    example: arr_ex.clone(),
                    ref_: None,
                };
                (SchemaRef::Inline(Box::new(schema)), arr_ex)
            }
            SorobanType::Result { ok_type, err_type: _ } => self.type_to_schema_and_example(ok_type),
            SorobanType::Custom { name } => {
                let st = SorobanType::from_type_string(name);
                if !matches!(st, SorobanType::Custom { name: ref n } if n == name) {
                    return self.type_to_schema_and_example(&st);
                }
                (
                    SchemaRef::Inline(Box::new(Schema {
                        r#type: Some("object".to_string()),
                        format: None,
                        description: Some(name.clone()),
                        properties: None,
                        required: None,
                        items: None,
                        additional_properties: None,
                        nullable: None,
                        example: Some(serde_json::Value::Object(serde_json::Map::new())),
                        ref_: None,
                    })),
                    Some(serde_json::Value::Object(serde_json::Map::new())),
                )
            }
        }
    }

    fn into_components(self) -> OpenApiComponents {
        OpenApiComponents {
            schemas: if self.schemas.is_empty() {
                None
            } else {
                Some(self.schemas)
            },
        }
    }
}

fn sanitize_schema_name(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect();
    if s.is_empty() {
        "Unnamed".to_string()
    } else {
        s
    }
}

/// Serialize OpenAPI doc to YAML string
pub fn to_yaml(doc: &OpenApiDoc) -> Result<String, serde_yaml::Error> {
    serde_yaml::to_string(doc)
}

/// Serialize OpenAPI doc to JSON string
pub fn to_json(doc: &OpenApiDoc) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(doc)
}
