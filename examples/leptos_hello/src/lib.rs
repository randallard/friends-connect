use leptos::*;
use leptos::prelude::*;
use wasm_bindgen_test::*;
use uuid::Uuid;
use web_sys::{js_sys, Storage, Response};
use reqwasm::http::Request;
use serde::{Deserialize, Serialize};

wasm_bindgen_test_configure!(run_in_browser);

#[derive(Serialize, Deserialize, Debug, Clone)]
struct UuidResponse {
    uuid: String,
}

#[component]
fn App() -> impl IntoView {
    log::debug!("Log from App component");
    let (data, set_data) = create_signal(String::new());
    let (uuid, set_uuid) = create_signal(String::new());

    let uuid_resource = create_resource(
        || (),  // Empty source signal - we only need to run this once
        |_| async move {
            let window = web_sys::window().expect("no global `window` exists");
            let storage = window.local_storage()
                .expect("failed to get localStorage")
                .expect("localStorage is not available");

            let response = match storage.get_item("user_uuid").unwrap() {
                Some(existing_uuid) => {
                    log::debug!("Using existing UUID: {}", existing_uuid);
                    UuidResponse { uuid: existing_uuid }
                },
                None => {
                    let new_uuid = Uuid::new_v4().to_string();
                    log::debug!("Generated new UUID: {}", new_uuid);
                    storage.set_item("user_uuid", &new_uuid)
                        .expect("failed to set UUID in localStorage");
                    UuidResponse { uuid: new_uuid }
                }
            };
            
            // Set the UUID signal with the result
            set_uuid.set(response.uuid.clone());
            response
        }
    );
    
    view! { 
        <div>"Hello"</div>
        {move || match uuid_resource.get() {
            Some(_) => view! { 
                <div data-testid="uuid">{uuid}</div>
                <div data-testid="data">{data}</div>
            }.into_any(),
            None => view! { <div>"Loading..."</div> }.into_any()   
        }}
    }
}

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_app_says_hello() {
    mount_to_body(|| view! { <App/> });
    
    // Get the div element
    let div = document()
        .query_selector("div")
        .unwrap()
        .unwrap();
        
    assert_eq!(div.text_content().unwrap(), "Hello");
}

#[wasm_bindgen_test]
fn test_app_shows_uuid() {
    mount_to_body(|| view! { <App/> });
    
    let uuid_div = document()
        .query_selector("div[data-testid='uuid']")
        .expect("query_selector should return Some")
        .expect("Should find UUID div");

    let uuid_text = uuid_div.text_content().unwrap();
        
    // Case-insensitive pattern using (?i) prefix
    let uuid_pattern = r"(?i)^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$";
      
    // Try parsing it as a UUID to validate
    assert!(Uuid::parse_str(&uuid_text).is_ok(), 
        "Text '{}' should be a valid UUID", uuid_text);
}

#[wasm_bindgen_test]
fn test_local_storage_access() {
    mount_to_body(|| view! { <App/> });
    
    // Get window.localStorage
    let window = web_sys::window().expect("no global `window` exists");
    let storage = window.local_storage()
        .expect("failed to get localStorage")
        .expect("localStorage is not available");
        
    // Try to set and get a value
    storage.set_item("test_key", "test_value")
        .expect("failed to set localStorage item");
        
    let value = storage.get_item("test_key")
        .expect("failed to get localStorage item")
        .expect("test_key not found in localStorage");
        
    assert_eq!(value, "test_value");
}

#[wasm_bindgen_test]
fn test_uuid_saved_to_local_storage() {
    wasm_logger::init(wasm_logger::Config::default());
    log::debug!("log from test_uuid_saved_to_local_storage");
    // Clear localStorage first to ensure clean state
    let window = web_sys::window().expect("no global `window` exists");
    let storage = window.local_storage()
        .expect("failed to get localStorage")
        .expect("localStorage is not available");
    storage.clear().expect("failed to clear localStorage");
    
    // First render - should generate new UUID
    mount_to_body(|| view! { <App/> });
            
    // Get the UUID displayed in the DOM
    let uuid_div = document()
        .query_selector("div[data-testid='uuid']")
        .expect("query_selector should return Some")
        .expect("Should find UUID div");
    let first_uuid = uuid_div.text_content().unwrap();
    
    // Store UUID for comparison
    let stored_uuid = storage.get_item("user_uuid")
        .expect("failed to get localStorage item")
        .expect("uuid not found in localStorage");
        
    assert_eq!(first_uuid, stored_uuid,
        "UUID in localStorage should match displayed UUID");

    // Remove first instance of component from document
    document().body().unwrap().set_inner_html("");
    
    // Mount a second time to check if the same UUID is displayed
    mount_to_body(|| view! { <App/> });
    
    let uuid_div = document()
        .query_selector("div[data-testid='uuid']")
        .expect("query_selector should return Some")
        .expect("Should find UUID div");
    let second_uuid = uuid_div.text_content().unwrap();
    
    // Second render should show same UUID
    assert_eq!(first_uuid, second_uuid, 
        "Second render should reuse UUID from localStorage");
}
