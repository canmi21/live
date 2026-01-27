/* tests/loader_tests.rs */

#![cfg(all(feature = "loader", feature = "json", feature = "validate"))]

use live::loader::{DynLoader, LoadResult, MemorySource, format::AnyFormat};
use serde::Deserialize;
use validator::Validate;

#[derive(Deserialize, Debug, PartialEq, Validate)]
struct Config {
	#[validate(length(min = 1))]
	foo: String,
}

impl live::loader::PreProcess for Config {}

#[tokio::test]
async fn test_dyn_loader_json() {
	let mut source = MemorySource::new();
	source.insert("test.json", b"{\"foo\": \"bar\"}".to_vec());

	let loader = DynLoader::builder()
		.source(source)
		.format(AnyFormat::Json)
		.build()
		.unwrap();

	let res = loader.load::<Config>("test").await;
	match res {
		LoadResult::Ok { value, .. } => assert_eq!(value.foo, "bar"),
		_ => panic!("Load failed"),
	}
}

#[tokio::test]
async fn test_dyn_loader_not_found() {
	let source = MemorySource::new();
	let loader = DynLoader::builder()
		.source(source)
		.format(AnyFormat::Json)
		.build()
		.unwrap();

	let res = loader.load::<Config>("missing").await;
	match res {
		LoadResult::NotFound => (),
		_ => panic!("Expected NotFound"),
	}
}
