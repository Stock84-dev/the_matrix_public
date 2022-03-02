use bitflags::bitflags;
use reqwest::Method;

const EXPIRES: i64 = 5;

bitflags! {
    pub struct PreProcess: u8 {
        const StringifyKeys = 0b00000001;
    }
}

#[derive(Debug)]
pub struct Schema {
    pub root_url: Option<String>,
    pub definitions: Vec<DefinitionMethod>,
    pub samples: Vec<DefinitionSample>,
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
}
//
// impl Schema {
//     pub fn write_code<Client, P>(&mut self, dir: P, api_name: &str) -> Result<()>
//     where
//         Client: Requestable,
//         P: AsRef<OsStr>,
//     {
//         self.fix_common_errors();
//         let snake_name = api_name.to_snake_case();
//         let dir = Path::new(&dir).join(&snake_name);
//         let src = dir.join("src");
//         std::fs::create_dir_all(&src)?;
//
//         info!("Generating project structure...");
//         Schema::write_cargo_toml(&dir, &snake_name)?;
//         self.write_lib_rs(&src, &snake_name)?;
//         info!("Project structure done");
//
//         let client = if self.api_key.is_some() {
//             Client::with_credentials(
//                 self.api_key.take().unwrap(),
//                 self.api_secret.take().unwrap(),
//             )
//         } else {
//             Client::default()
//         };
//         let mut definitions = OpenOptions::new()
//             .write(true)
//             .create(true)
//             .open(src.join("definitions.rs"))?;
//
//         let mut requests = OpenOptions::new()
//             .write(true)
//             .create(true)
//             .open(src.join("requests.rs"))?;
//
//         definitions.write(
//             b"use rust_decimal::Decimal;
// use serde::{Deserialize, Serialize};\n\n",
//         )?;
//         requests.write_all(
//             b"use rust_decimal::Decimal;
// use super::definitions::*;
// use request::{Method, Request};\n\n",
//         )?;
//
//         info!("Generating from definitions...");
//         for (i, definition) in self.definitions.iter().enumerate() {
//             let code = definition.to_method_code(&client)?;
//             definitions.write_all(code.definition.as_bytes())?;
//             requests.write_all(code.request.as_bytes())?;
//             trace!(
//                 "Finished generating definition {} / {}",
//                 i + 1,
//                 self.definitions.len()
//             );
//         }
//         info!("Definitions done");
//         info!("Generating from samples...");
//         for (i, sample) in self.samples.iter().enumerate() {
//             let code = sample.to_method_code()?;
//             definitions.write_all(code.definition.as_bytes())?;
//             requests.write_all(code.request.as_bytes())?;
//             trace!(
//                 "Finished generating definition {} / {}",
//                 i + 1,
//                 self.samples.len()
//             );
//         }
//         info!("Samples done");
//         Ok(())
//     }
//
//     fn write_cargo_toml(dir: &Path, snake_name: &str) -> Result<()> {
//         let mut cargo = OpenOptions::new()
//             .write(true)
//             .create(true)
//             .open(dir.join("Cargo.toml"))?;
//         let template = format!(
//             r#"[package]
// name = "{}"
// version = "0.1.0"
// authors = ["Stock84-dev <leontk8@gmail.com>"]
// edition = "2018"
//
// [features]
// schema = ["request/schema"]
//
// # See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html
//
// [dependencies]
// request = {{ path = "../request" }}
// rust_decimal = "1.8.1"
// serde = {{ version = "1.0.104", features = ["derive"] }}
//
// [dev-dependencies]
// request = {{ path = "../request", features = ["schema"] }}
// "#,
//             snake_name
//         );
//         Ok(cargo.write_all(template.as_bytes())?)
//     }
//
//     fn write_lib_rs(&self, src: &Path, snake_name: &str) -> Result<()> {
//         let client_type = format!("{}Client", snake_name.to_pascal_case());
//         let mut lib = OpenOptions::new()
//             .write(true)
//             .create(true)
//             .open(src.join("lib.rs"))?;
//         let template = format!(
//             r#"#[macro_use]
// extern crate serde;
//
// pub use request;
// use request::*;
//
// pub mod definitions;
// pub mod requests;
//
// #[derive(Default)]
// pub struct {0} {{
//     credentials: Option<Credentials>,
//     client: ReqwestClient,
// }}
//
// impl Requestable for {0} {{
//     fn with_credentials(api_key: String, api_secret: String) -> Self {{
//         Self {{
//             credentials: Some(Credentials {{
//                 api_key,
//                 api_secret,
//             }}),
//             client: request::Client::builder().build().unwrap(),
//         }}
//     }}
//     fn get_credentials(&self) -> &Option<Credentials> {{
//         &self.credentials
//     }}
//
//     fn get_client(&self) -> &ReqwestClient {{
//         &self.client
//     }}
//
//     fn build_request(builder: RequestBuilder, headers: RequestHeaders) -> RequestBuilder {{
//         match headers {{
//             RequestHeaders::Public => {{
//                 builder
//             }},
//             RequestHeaders::Private(headers) => {{
//                 builder
//             }},
//         }}
//     }}
// }}
//
//
// #[cfg(feature = "schema")]
// pub mod schema {{
//     pub use request::schema::*;
//     pub use request::*;
//
//     pub fn schema() -> Schema {{
// {1}
//     }}
// }}
//
// "#,
//             client_type,
//             self.to_string()
//         );
//         Ok(lib.write_all(template.as_bytes())?)
//     }
//
//     fn to_string(&self) -> String {
//         let mut definitions = String::new();
//         for definition in &self.definitions {
//             definitions.push_str(&definition.to_string());
//             definitions.push_str(",\n");
//         }
//         let mut samples = String::new();
//         for sample in &self.samples {
//             samples.push_str(&sample.to_string());
//             samples.push_str(",\n");
//         }
//         let mut schema = format!(
//             "       Schema {{
//             root_url: None,
//             definitions: vec![
// {}
//             ],
//             samples: vec![
// {}
//             ],
//             api_key: None,
//             api_secret: None,
//         }}",
//             definitions, samples
//         );
//
//         schema
//     }
//
//     fn fix_common_errors(&mut self) {
//         if let Some(root) = &self.root_url {
//             for definition in &mut self.definitions {
//                 definition.endpoint = format!("{}{}", root, &definition.endpoint);
//             }
//         }
//
//         for definition in &mut self.definitions {
//             if definition.pre_process.contains(PreProcess::StringifyKeys) {
//                 definition.payload = stringify_keys(&definition.payload);
//             }
//             let mut url = Url::parse(&definition.endpoint).unwrap();
//             let mut json = String::from("{\n");
//             let count = url.query_pairs().count();
//             if count != 0 {
//                 for (i, (name, value)) in url.query_pairs().enumerate() {
//                     json.push('"');
//                     json.push_str(&*name);
//                     json.push_str(r#"":"#);
//                     if value == "true" {
//                         json.push_str("true");
//                     } else if value == "false" {
//                         json.push_str("false");
//                     } else {
//                         match Decimal::from_str(&value) {
//                             Ok(_) => json.push_str(&*value),
//                             Err(_) => {
//                                 json.push('"');
//                                 json.push_str(&*value);
//                                 json.push('"');
//                             }
//                         };
//                     };
//                     if i != count - 1 {
//                         json.push_str(",\n");
//                     }
//                 }
//                 json.push_str("\n}");
//                 if !definition.payload.is_empty() {
//                     panic!("Payload already contains elements, tried to insert from url");
//                 }
//                 definition.payload = json;
//             }
//         }
//         for sample in &mut self.samples {
//             if sample.pre_process.contains(PreProcess::StringifyKeys) {
//                 sample.payload = stringify_keys(&sample.payload);
//                 sample.response = stringify_keys(&sample.response);
//             }
//         }
//     }
// }
//
#[derive(Debug)]
pub struct DefinitionSample {
    pub endpoint: String,
    pub method: Method,
    pub payload: String,
    pub response: String,
    pub is_signed: bool,
    pub pre_process: PreProcess,
}
//
// impl DefinitionSample {
//     fn from_method(method: &DefinitionMethod, response: String) -> DefinitionSample {
//         DefinitionSample {
//             endpoint: method.endpoint.clone(),
//             method: method.method.clone(),
//             payload: method.payload.clone(),
//             response,
//             is_signed: method.is_signed,
//             pre_process: method.pre_process,
//         }
//     }
//
//     fn to_string(&self) -> String {
//         format!(
//             r##"                DefinitionSample {{
//                     endpoint: "{}".into(),
//                     method: Method::{},
//                     payload: r#"{}"#.into(),
//                     response: r#"{}"#.into(),
//                     is_signed: {},
//                     pre_process: PreProcess::empty(),
//                 }}"##,
//             self.endpoint,
//             self.method.as_str().to_screaming_snake_case(),
//             self.payload.trim(),
//             self.response.trim(),
//             if self.is_signed { "true" } else { "false" }
//         )
//     }
//
//     pub fn to_method_code(&self) -> Result<MethodCode> {
//         // TODO: does this always clone?
//         debug!("payload = |{}|", self.payload);
//         debug!("response = |{}|", self.response);
//         let response = if self.response.is_empty() {
//             Structure::from_value(Value::Null, self.get_name_from_url())
//         } else {
//             let value = serde_json::from_str(&self.response).context(self.response.clone())?;
//             Structure::from_value(value, self.get_name_from_url())
//         };
//         let (response_name, response_type) = DefinitionMethod::response_name_and_type(&response);
//         let definition = response.to_definition();
//         let request = self.to_request(&response_name, &response_type)?;
//
//         Ok(MethodCode {
//             definition,
//             request,
//         })
//     }
//
//     fn to_request(&self, response_name: &str, response_type: &str) -> Result<String> {
//         let request_name = format!("{}{}", self.method.as_str().to_pascal_case(), response_name);
//         let mut request = format!(
//             r#"impl Request for {} {{
//     const METHOD: Method = Method::{};
//     const URL: &'static str = "{}";
//     type Response = {};
// }}
// "#,
//             request_name,
//             self.method.as_str().to_screaming_snake_case(),
//             match self.endpoint.rfind('?') {
//                 Some(i) => &self.endpoint[..i],
//                 None => &self.endpoint,
//             },
//             response_type,
//         );
//         if self.payload.is_empty() {
//             request.insert_str(
//                 0,
//                 &format!(
//                     "\n#[derive(Clone, Debug, Default, Deserialize, Serialize)]
// pub struct {};\n",
//                     request_name
//                 ),
//             );
//         } else {
//             let value = serde_json::from_str(&self.payload)?;
//             let payload = Structure::from_value(value, request_name);
//             let mut definition = payload.to_definition();
//             request.insert_str(0, &definition);
//         };
//         Ok(request)
//     }
//
//     fn get_name_from_url(&self) -> String {
//         let url = &self.endpoint;
//         let name = match url.rfind("?") {
//             None => {
//                 let i = url.rfind("/").unwrap() + 1;
//                 if i == url.len() {
//                     let start = url[..i - 1].rfind("/").unwrap() + 1;
//                     &url[start..i - 1]
//                 } else {
//                     &url[i..]
//                 }
//             }
//             Some(mut i) => {
//                 if url.chars().skip(i - 1).next().unwrap() == '/' {
//                     i -= 1;
//                 }
//                 let slice = &url[url[..i].rfind("/").unwrap() + 1..i];
//                 slice
//             }
//         };
//         name.to_pascal_case()
//     }
// }
//
#[derive(Debug)]
pub struct DefinitionMethod {
    pub endpoint: String,
    pub method: Method,
    pub payload: String,
    pub is_signed: bool,
    pub pre_process: PreProcess,
}
//
// pub struct MethodCode {
//     pub definition: String,
//     pub request: String,
// }
//
// impl DefinitionMethod {
//     pub fn to_method_code<Client: Requestable>(&self, client: &Client) -> Result<MethodCode> {
//         let mut runtime = tokio::runtime::Runtime::new()?;
//         let json = runtime.block_on(request_definition(client, self))?;
//         let sample = DefinitionSample::from_method(&self, json);
//         sample.to_method_code()
//     }
//
//     fn to_string(&self) -> String {
//         format!(
//             r##"                DefinitionMethod {{
//                     endpoint: "{}".into(),
//                     method: Method::{},
//                     payload: r#"{}"#.into(),
//                     is_signed: {},
//                     pre_process: PreProcess::empty(),
//                 }}"##,
//             self.endpoint,
//             self.method.as_str().to_screaming_snake_case(),
//             self.payload.trim(),
//             if self.is_signed { "true" } else { "false" },
//         )
//     }
//
//     fn response_name_and_type(response: &Structure) -> (String, String) {
//         let mut response_type = response.name.to_pascal_case();
//         let response_name = match response.kind {
//             Kind::Array(_) => {
//                 response_type = format!("Vec<{}>", &response_type[..response_type.len() - 1]);
//                 format!("{}", response.name)
//             }
//             _ => response.name.clone(),
//         };
//         (response_name, response_type)
//     }
// }
//
// #[derive(Clone, Debug, PartialEq)]
// pub enum Kind {
//     Object(Vec<Structure>),
//     Array(Box<Structure>),
//     Number,
//     Bool,
//     String,
// }
//
// #[derive(Clone, Debug, PartialEq)]
// pub struct Structure {
//     name: String,
//     kind: Kind,
//     optional: bool,
// }
//
// impl Structure {
//     pub fn from_value(value: Value, name: String) -> Structure {
//         match value {
//             Value::Null => Structure::new(name, Kind::Object(Vec::new())),
//             Value::Bool(_) => Structure::new(name, Kind::Bool),
//             Value::Number(_) => Structure::new(name, Kind::Number),
//             Value::String(s) => match Decimal::from_str(&s) {
//                 Ok(_) => Structure::new(name, Kind::Number),
//                 Err(_) => Structure::new(name, Kind::String),
//             },
//             Value::Array(a) => Structure::from_array(a, name),
//             Value::Object(o) => Structure::from_object(o, name),
//         }
//     }
//
//     pub fn to_definition(self) -> String {
//         let s = match self.kind {
//             Kind::Object(_) => self,
//             Kind::Array(s) => *s,
//             _ => panic!(),
//         };
//         let mut definition =
//             String::from("\n#[derive(Clone, Debug, Default, Deserialize, Serialize)]\n");
//         definition.push_str("pub struct ");
//         definition.push_str(&s.name.to_pascal_case());
//
//         let structs = match s.kind {
//             Kind::Object(o) => o,
//             _ => panic!(),
//         };
//
//         if structs.is_empty() {
//             definition.push_str(";\n");
//         } else {
//             definition.push_str(" {\n");
//             for s in structs {
//                 let name = s.name.clone();
//                 let optional = s.optional;
//                 let mut kind = match s.kind {
//                     Kind::Number => "Decimal".into(),
//                     Kind::Bool => "bool".into(),
//                     Kind::String => "String".into(),
//                     Kind::Array(_) | Kind::Object(_) => {
//                         let kind = s.name.to_pascal_case();
//                         definition.insert_str(0, &s.to_definition());
//                         kind
//                     }
//                 };
//                 let name = if !name.is_snake_case() {
//                     Structure::rename(&mut definition, &name);
//                     name.to_snake_case()
//                 } else if name == "type" {
//                     Structure::rename(&mut definition, "type");
//                     "kind".into()
//                 } else {
//                     name
//                 };
//                 definition.push_str("   pub ");
//                 definition.push_str(&name);
//                 definition.push_str(": ");
//                 if optional {
//                     definition.push_str("Option<");
//                     definition.push_str(&kind);
//                     definition.push('>');
//                 } else {
//                     definition.push_str(&kind);
//                 }
//                 definition.push_str(",\n");
//             }
//             definition.push_str("}\n");
//         }
//
//         definition
//     }
//
//     fn rename(definition: &mut String, name: &str) {
//         definition.push_str("   #[serde(rename = \"");
//         definition.push_str(&name);
//         definition.push_str("\")]\n");
//     }
//
//     fn from_object(object: Map<String, Value>, name: String) -> Structure {
//         let mut structs = Vec::with_capacity(object.len());
//         for (k, v) in object {
//             structs.push(Structure::from_value(v, k));
//         }
//         Structure::new(name, Kind::Object(structs))
//     }
//
//     fn from_array(array: Vec<Value>, collection_name: String) -> Structure {
//         let object_name = &collection_name[..collection_name.len() - 1];
//         let mut master = Structure::new(object_name.into(), Kind::Bool);
//         for value in array {
//             let s = Structure::from_value(value, object_name.into());
//             if std::mem::discriminant(&master.kind) != std::mem::discriminant(&s.kind) {
//                 master = s.clone();
//             }
//             match &mut master.kind {
//                 Kind::Object(o) => {
//                     let slave = s.get_object();
//                     for mut so in slave {
//                         if !o.contains(&so) {
//                             so.optional = true;
//                             o.push(so);
//                         }
//                     }
//                 }
//                 _ => {}
//             }
//         }
//         Structure {
//             name: collection_name,
//             kind: Kind::Array(Box::new(master)),
//             optional: false,
//         }
//     }
//
//     fn get_object(self) -> Vec<Structure> {
//         match self.kind {
//             Kind::Object(o) => o,
//             _ => panic!(),
//         }
//     }
//
//     fn new(name: String, kind: Kind) -> Structure {
//         Structure {
//             name,
//             kind,
//             optional: false,
//         }
//     }
// }
//
// async fn request_definition<C>(client: &C, definition_method: &DefinitionMethod) -> Result<String>
// where
//     C: Requestable,
// {
//     let expires = (Utc::now() + Duration::seconds(EXPIRES)).timestamp();
//     let url = Url::parse(&definition_method.endpoint)?;
//     let mut builder = client
//         .client()
//         .request(definition_method.method.clone(), url.clone());
//     let body = match definition_method.method {
//         Method::PUT | Method::POST => definition_method.payload.clone(),
//         _ => "".to_string(),
//     };
//     builder = if definition_method.is_signed {
//         let (key, signature) =
//             client.signature(definition_method.method.clone(), expires, &url, &body)?;
//         client.build_request(
//             builder,
//             RequestHeaders::Private(SignedRequestHeaders {
//                 expires,
//                 signature: &signature,
//                 key,
//             }),
//         )
//     } else {
//         client.build_request(builder, RequestHeaders::Public)
//     };
//     if !body.is_empty() {
//         debug!("body = |{}|", body);
//         builder = builder.body(body);
//     }
//
//     let resp = builder.send().await?;
//     Ok(resp.text().await?)
// }
//
// fn stringify_keys(string: &str) -> String {
//     if string.is_empty() {
//         return String::new();
//     }
//     let mut result = String::with_capacity(string.len());
//     let mut iter = string.chars();
//     let mut prev = iter.next().unwrap();
//
//     for line in string.lines() {
//         if line.contains(':') {
//             let mut is_key = false;
//             let mut finished = false;
//             for c in line.chars() {
//                 if !finished && prev.is_whitespace() && !c.is_whitespace() {
//                     is_key = true;
//                     result.push('"');
//                 } else if !finished && is_key && c == ':' {
//                     result.push('"');
//                     finished = true;
//                 }
//                 result.push(c);
//                 prev = c;
//             }
//         } else {
//             result.push_str(line);
//         }
//         result.push('\n');
//     }
//     result
// }
