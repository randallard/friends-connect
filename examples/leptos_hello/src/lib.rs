use leptos::*;
use leptos::prelude::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[component]
fn App() -> impl IntoView {
    view! { <div>"Hello"</div> }
}

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
        
    assert!(uuid_text.matches(r"^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$").count() == 1,
        "Text '{}' should be a valid UUID v4", uuid_text);
}