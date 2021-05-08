use crate::driver::KeyCode;
use anyhow::Result;
use graphql_client::*;
use lazy_static::*;
use log::*;
use serde_json::Value;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum QueryError {
    #[error("Missing data in query response")]
    MissingDataInResponse,
    #[error("Failed to parse keyboard name")]
    ParseKeyboardError,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
enum KeyboardType {
    Ergodox,
    Moonlander,
    Planck,
}

impl FromStr for KeyboardType {
    type Err = QueryError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ergodox-ez" => Ok(KeyboardType::Ergodox),
            // TODO: Make sure what this one actually is
            "moonlander" => Ok(KeyboardType::Moonlander),
            // TODO: Make sure what this one actually is
            "planck" => Ok(KeyboardType::Planck),
            _ => Err(QueryError::ParseKeyboardError),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Layout {
    keyboard: Option<KeyboardType>,
    title: String,
    model: String,
    layers: Vec<Layer>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Layer {
    name: String,
    position: usize,
    color: Option<String>,
    keys: Vec<Key>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Key {
    color: Option<String>,
    key_code: Option<String>,
    layer: Option<i64>,
    command: Option<String>,
    modifiers: Option<String>,
}

impl Layout {
    pub fn build_from_query_response(
        response: Response<layout_query::ResponseData>,
    ) -> Result<Self> {
        let layout = response
            .data
            .ok_or(QueryError::MissingDataInResponse)?
            .layout
            .ok_or(QueryError::MissingDataInResponse)?;
        let keyboard_type = KeyboardType::from_str(&layout.geometry.unwrap_or_default()).ok();
        let layout_title = layout.title;
        let keyboard_model = layout.revision.model;
        let mut layers = vec![];
        for layer in layout.revision.layers {
            let name = layer.title;
            let position = layer.position;
            let color = layer.color;
            let mut keys = vec![];
            for key in layer.keys.ok_or(QueryError::MissingDataInResponse)? {
                let color = key.get("color").and_then(|color| match color {
                    Value::String(color) => Some(color.to_owned()),
                    _ => None,
                });
                let key_code = key.get("code").and_then(|key_code| match key_code {
                    Value::String(key_code) => Some(key_code.to_owned()),
                    _ => None,
                });
                let layer = key.get("layer").and_then(|layer| match layer {
                    Value::Number(layer) => layer.as_i64(),
                    _ => None,
                });
                let command = key.get("command").and_then(|command| match command {
                    Value::String(command) => Some(command.to_owned()),
                    _ => None,
                });
                let modifiers = key.get("modifiers").and_then(|modifiers| match modifiers {
                    Value::String(modifiers) => Some(modifiers.to_owned()),
                    _ => None,
                });
                keys.push(Key {
                    color,
                    key_code,
                    layer,
                    command,
                    modifiers,
                });
            }
            layers.push(Layer {
                name,
                color,
                position: position as usize,
                keys,
            });
        }
        Ok(Layout {
            keyboard: keyboard_type,
            title: layout_title,
            model: keyboard_model,
            layers,
        })
    }

    pub fn get_key(&self, key: KeyCode, layer: usize) -> Option<&Key> {
        let key_index = ERGODOX_MAP
            .get(key.column as usize)
            .and_then(|column| column.get(key.row as usize))?;
        let key = self
            .layers
            .get(layer)
            .and_then(|layer| layer.keys.get(*key_index as usize))?;
        Some(key)
    }
}

type Json = Vec<std::collections::HashMap<String, Value>>;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "GraphQL/oryx_schema.json",
    query_path = "GraphQL/layout_query.graphql",
    response_derives = "Debug"
)]
pub struct LayoutQuery;

pub fn query_layout(hash_id: String, revision_id: String) -> Result<Layout> {
    let variables = layout_query::Variables {
        hash_id,
        revision_id,
    };
    let request_body = LayoutQuery::build_query(variables);

    let client = reqwest::blocking::Client::new();
    let res = client
        .post("https://oryx.ergodox-ez.com/graphql")
        .json(&request_body)
        .send()?;
    let response_body: Response<layout_query::ResponseData> = res.json()?;
    let layout = Layout::build_from_query_response(response_body)?;
    Ok(layout)
}

lazy_static! {
    static ref ERGODOX_MAP: Vec<Vec<i32>> = {
        vec![
            vec![0, 1, 2, 3, 4, 5, 6, 38, 39, 40, 41, 42, 43, 44],
            vec![7, 8, 9, 10, 11, 12, 13, 45, 46, 47, 48, 49, 50, 51],
            vec![14, 15, 16, 17, 18, 19, -1, -1, 52, 53, 54, 55, 56, 57],
            vec![20, 21, 22, 23, 24, 25, 26, 58, 59, 60, 61, 62, 63, 64],
            vec![27, 28, 29, 30, 31, -1, -1, -1, -1, 65, 66, 67, 68, 69],
            vec![-1, 37, 36, 35, 34, 32, 33, 70, 71, 72, 75, 74, 73],
        ]
    };
}
