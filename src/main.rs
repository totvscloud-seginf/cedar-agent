extern crate core;
extern crate rocket;

use std::borrow::Borrow;
use std::path::PathBuf;

use rocket::catchers;
use rocket::http::ContentType;
use rocket_okapi::settings::UrlObject;
use rocket_okapi::{openapi_get_routes, rapidoc::*, swagger_ui::*};

use crate::services::data::memory::MemoryDataStore;
use crate::services::data::DataStore;
use crate::services::policies::memory::MemoryPolicyStore;
use crate::services::policies::PolicyStore;
use std::sync::Arc;

mod authn;
mod common;
mod config;
mod errors;
mod logger;
mod routes;
mod schemas;
mod services;

#[rocket::main]
async fn main() {
    let config = config::init();
    logger::init(&config);
    let server_config: rocket::figment::Figment = config.borrow().into();
    let data_store = MemoryDataStore::new();
    let policy_store = MemoryPolicyStore::new();

    let data_store_arc = Arc::new(data_store);
    let policy_store_arc = Arc::new(policy_store);

    if !config.file_watcher.is_none() && config.file_watcher.unwrap_or(false) {
        services::file_watcher::init(
            config.data.clone().unwrap_or(PathBuf::new()), 
            config.policy.clone().unwrap_or(PathBuf::new()), 
            data_store_arc.clone(), 
            policy_store_arc.clone()
        );
    }

    let launch_result = rocket::custom(server_config)
        .attach(common::DefaultContentType::new(ContentType::JSON))
        .attach(services::data::load_from_file::InitDataFairing)
        .attach(services::policies::load_from_file::InitPoliciesFairing)
        .manage(config)
        .manage(Box::new(policy_store_arc.clone()) as Box<dyn PolicyStore>)
        .manage(Box::new(data_store_arc.clone()) as Box<dyn DataStore>)
        .manage(cedar_policy::Authorizer::new())
        .register(
            "/",
            catchers![
                errors::catchers::handle_500,
                errors::catchers::handle_404,
                errors::catchers::handle_400,
            ],
        )
        .mount(
            "/v1",
            openapi_get_routes![
                routes::healthy,
                routes::policies::get_policies,
                routes::policies::get_policy,
                routes::policies::create_policy,
                routes::policies::update_policies,
                routes::policies::update_policy,
                routes::policies::delete_policy,
                routes::data::get_entities,
                routes::data::update_entities,
                routes::data::delete_entities,
                routes::authorization::is_authorized,
            ],
        )
        .mount(
            "/swagger-ui/",
            make_swagger_ui(&SwaggerUIConfig {
                url: "../v1/openapi.json".to_owned(),
                ..Default::default()
            }),
        )
        .mount(
            "/rapidoc/",
            make_rapidoc(&RapiDocConfig {
                general: GeneralConfig {
                    spec_urls: vec![UrlObject::new("General", "../v1/openapi.json")],
                    ..Default::default()
                },
                hide_show: HideShowConfig {
                    allow_spec_url_load: false,
                    allow_spec_file_load: false,
                    ..Default::default()
                },
                ..Default::default()
            }),
        )
        .launch()
        .await;
    match launch_result {
        Ok(_) => println!("Rocket shut down gracefully."),
        Err(err) => println!("Rocket had an error: {}", err),
    };
}
